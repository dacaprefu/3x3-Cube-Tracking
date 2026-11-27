use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use chrono::Local;
use cubo_cfop::analytics::SolveAnalyzer;
use cubo_cfop::ble::{scan_for_gan_cubes, AlignedTimestamp, GanBleClient, GanEvent};
use cubo_cfop::cfop::{CfopPhase, CfopTracker};
use cubo_cfop::cube::{generate_wca_scramble, parse_moves, Color, Move};
use cubo_cfop::export::export_session_to_excel;
use cubo_cfop::gyro::{CubeOrientation, GyroProcessor};
use cubo_cfop::{
    analyze_session_with_gemini, analyze_solve_with_gemini, Cube, SolveReport,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BleConnectionState {
    pub connected: bool,
    pub battery: Option<u8>,
    pub mac_address: Option<String>,
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScrambleState {
    pub moves: Vec<String>,
    pub completed_count: usize,
    pub correction_move: Option<String>,
    pub is_scrambled_ready: bool,
    pub has_prev: bool,
    pub is_reviewing_previous: bool,
}

impl ScrambleState {
    pub fn from_moves(moves: Vec<String>, has_prev: bool, is_reviewing_previous: bool) -> Self {
        Self {
            moves,
            completed_count: 0,
            correction_move: None,
            is_scrambled_ready: false,
            has_prev,
            is_reviewing_previous,
        }
    }

    pub fn new_random(has_prev: bool) -> Self {
        Self::from_moves(generate_wca_scramble(20), has_prev, false)
    }

    pub fn as_text(&self) -> String {
        self.moves.join(" ")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionData {
    pub name: String,
    pub solves: Vec<SolveReport>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    pub ble: BleConnectionState,
    pub sessions: Vec<SessionData>,
    pub active_session: String,
    pub demo_mode: bool,
    pub scramble: ScrambleState,
    pub scramble_history: Vec<Vec<String>>,
    pub scramble_index: usize,
}

impl AppState {
    pub fn active_solves_mut(&mut self) -> &mut Vec<SolveReport> {
        if let Some(pos) = self
            .sessions
            .iter()
            .position(|s| s.name == self.active_session)
        {
            return &mut self.sessions[pos].solves;
        }
        let fallback_name = if self.active_session.is_empty() {
            "Serie 1".to_string()
        } else {
            self.active_session.clone()
        };
        self.active_session = fallback_name.clone();
        self.sessions.push(SessionData {
            name: fallback_name,
            solves: Vec::new(),
        });
        let last_idx = self.sessions.len().saturating_sub(1);
        &mut self.sessions[last_idx].solves
    }

    pub fn advance_to_new_scramble(&mut self) -> ScrambleState {
        let moves = generate_wca_scramble(20);
        self.scramble_history.push(moves.clone());
        self.scramble_index = self.scramble_history.len().saturating_sub(1);
        let next = ScrambleState::from_moves(moves, self.scramble_index > 0, false);
        self.scramble = next.clone();
        next
    }
}

pub type SharedAppState = Arc<Mutex<AppState>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeMovePayload {
    pub cube_move: Move,
    pub move_notation: String,
    pub stickers: Vec<Color>,
    pub phase: CfopPhase,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CubeTelemetryPayload {
    pub battery: Option<u8>,
    pub orientation: CubeOrientation,
    pub quaternion: [f32; 4],
    pub timestamp_ms: u64,
}

#[tauri::command]
fn ping_backend() -> String {
    let cube = Cube::new();
    let solved = cubo_cfop::is_cube_solved(&cube);
    format!("pong from cubo_cfop backend (cube solved: {solved})")
}

#[tauri::command]
async fn get_current_scramble(state: State<'_, SharedAppState>) -> Result<ScrambleState, String> {
    let guard = state.lock().await;
    Ok(guard.scramble.clone())
}

#[tauri::command]
async fn next_scramble(
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<String, String> {
    let next_state = {
        let mut guard = state.lock().await;
        if guard.scramble_index + 1 < guard.scramble_history.len() {
            guard.scramble_index += 1;
            let idx = guard.scramble_index;
            let moves = guard.scramble_history[idx].clone();
            let reviewing = idx + 1 < guard.scramble_history.len();
            let st = ScrambleState::from_moves(moves, idx > 0, reviewing);
            guard.scramble = st.clone();
            st
        } else {
            guard.advance_to_new_scramble()
        }
    };
    let text = next_state.as_text();
    let _ = app.emit("scramble-updated", &next_state);
    Ok(text)
}

#[tauri::command]
async fn prev_scramble(
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<Option<String>, String> {
    let prev_state = {
        let mut guard = state.lock().await;
        if guard.scramble_index == 0 || guard.scramble_history.is_empty() {
            return Ok(None);
        }
        guard.scramble_index -= 1;
        let idx = guard.scramble_index;
        let moves = guard.scramble_history[idx].clone();
        let st = ScrambleState::from_moves(moves, idx > 0, true);
        guard.scramble = st.clone();
        st
    };
    let text = prev_state.as_text();
    let _ = app.emit("scramble-updated", &prev_state);
    Ok(Some(text))
}

#[tauri::command]
async fn start_ble_scan(
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<(), String> {
    let shared_state = Arc::clone(state.inner());

    tokio::spawn(async move {
        let discovered = match scan_for_gan_cubes(Duration::from_secs(5)).await {
            Ok(list) => list,
            Err(err) => {
                eprintln!("Error escaneando cubos GAN: {err}");
                return;
            }
        };

        let Some(target) = discovered.into_iter().next() else {
            eprintln!("No se encontró ningún cubo GAN durante el escaneo.");
            return;
        };

        let mac_str = format!(
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            target.mac[0],
            target.mac[1],
            target.mac[2],
            target.mac[3],
            target.mac[4],
            target.mac[5]
        );
        let device_name = target.name.clone();

        let client = match GanBleClient::connect(target).await {
            Ok(c) => c,
            Err(err) => {
                eprintln!("Error conectando al cubo GAN: {err}");
                return;
            }
        };

        let (tx, mut rx) = tokio::sync::mpsc::channel::<(GanEvent, AlignedTimestamp)>(200);
        if let Err(err) = client.start_event_stream(tx).await {
            eprintln!("Error iniciando stream de eventos BLE: {err}");
            return;
        }

        {
            let mut guard = shared_state.lock().await;
            guard.ble.connected = true;
            guard.ble.mac_address = Some(mac_str);
            guard.ble.device_name = Some(device_name);
        }

        let mut tracker = CfopTracker::new();
        let mut gyro_processor = GyroProcessor::new();
        let analyzer = SolveAnalyzer::new();
        let mut scrambling_phase = true;
        let mut scramble_cube = Cube::new();
        let mut wrong_move_stack: Vec<Move> = Vec::new();
        let mut last_scramble_moves: Vec<String> = Vec::new();

        while let Some((event, aligned)) = rx.recv().await {
            match event {
                GanEvent::Move(m_event) => {
                    let m = m_event.to_cube_move();
                    let ts = aligned.elapsed_solve_ms;

                    if scrambling_phase {
                        {
                            let guard = shared_state.lock().await;
                            if guard.scramble.moves != last_scramble_moves {
                                last_scramble_moves = guard.scramble.moves.clone();
                                scramble_cube = Cube::new();
                                wrong_move_stack.clear();
                            }
                        }
                        scramble_cube.apply_move(m);
                        let (updated_scramble, ready_moves) = {
                            let mut guard = shared_state.lock().await;
                            let parsed = parse_moves(&guard.scramble.as_text()).unwrap_or_default();
                            let mut expected_cube = Cube::new();
                            expected_cube.apply_moves(&parsed);

                            if scramble_cube == expected_cube && !parsed.is_empty() {
                                guard.scramble.completed_count = parsed.len();
                                guard.scramble.correction_move = None;
                                guard.scramble.is_scrambled_ready = true;
                                (guard.scramble.clone(), Some(parsed))
                            } else {
                                if wrong_move_stack.is_empty() {
                                    let idx = guard.scramble.completed_count;
                                    if idx < parsed.len() && m == parsed[idx] {
                                        guard.scramble.completed_count += 1;
                                        if guard.scramble.completed_count == parsed.len()
                                            || scramble_cube == expected_cube
                                        {
                                            guard.scramble.is_scrambled_ready = true;
                                            guard.scramble.correction_move = None;
                                            (guard.scramble.clone(), Some(parsed))
                                        } else {
                                            (guard.scramble.clone(), None)
                                        }
                                    } else {
                                        wrong_move_stack.push(m);
                                        guard.scramble.correction_move =
                                            Some(m.inverse().to_string());
                                        (guard.scramble.clone(), None)
                                    }
                                } else if wrong_move_stack.last().copied() == Some(m.inverse()) {
                                    wrong_move_stack.pop();
                                    guard.scramble.correction_move = wrong_move_stack
                                        .last()
                                        .map(|w| w.inverse().to_string());
                                    if wrong_move_stack.is_empty()
                                        && (guard.scramble.completed_count == parsed.len()
                                            || scramble_cube == expected_cube)
                                        && !parsed.is_empty()
                                    {
                                        guard.scramble.is_scrambled_ready = true;
                                        (guard.scramble.clone(), Some(parsed))
                                    } else {
                                        (guard.scramble.clone(), None)
                                    }
                                } else {
                                    wrong_move_stack.push(m);
                                    guard.scramble.correction_move =
                                        Some(m.inverse().to_string());
                                    (guard.scramble.clone(), None)
                                }
                            }
                        };

                        let _ = app.emit("scramble-updated", &updated_scramble);
                        let phase_to_emit = if let Some(parsed_moves) = ready_moves {
                            scrambling_phase = false;
                            wrong_move_stack.clear();
                            tracker = CfopTracker::new();
                            tracker.apply_scramble(&parsed_moves);
                            CfopPhase::Inspection
                        } else {
                            CfopPhase::Scramble
                        };

                        let payload = CubeMovePayload {
                            cube_move: m,
                            move_notation: m.to_string(),
                            stickers: scramble_cube.to_stickers().to_vec(),
                            phase: phase_to_emit,
                            timestamp_ms: ts,
                        };
                        let _ = app.emit("cube-move", &payload);
                        continue;
                    }

                    tracker.on_move(m, ts);

                    let payload = CubeMovePayload {
                        cube_move: m,
                        move_notation: m.to_string(),
                        stickers: tracker.cube().to_stickers().to_vec(),
                        phase: tracker.current_phase(),
                        timestamp_ms: ts,
                    };
                    let _ = app.emit("cube-move", &payload);

                    if tracker.current_phase() == CfopPhase::Solved {
                        if let Ok(report) = analyzer.analyze(&tracker) {
                            let next = {
                                let mut guard = shared_state.lock().await;
                                guard.active_solves_mut().push(report.clone());
                                guard.advance_to_new_scramble()
                            };
                            let _ = app.emit("solve-completed", &report);
                            let _ = app.emit("scramble-updated", &next);
                        }
                        tracker = CfopTracker::new();
                        scrambling_phase = true;
                        scramble_cube = Cube::new();
                        wrong_move_stack.clear();
                    }
                }
                GanEvent::Gyro(g_event) => {
                    let ts = aligned.elapsed_solve_ms;
                    let _ = gyro_processor.process_quaternion(g_event.quaternion, ts);
                    let battery = {
                        let guard = shared_state.lock().await;
                        guard.ble.battery
                    };
                    let q = g_event.quaternion;
                    let payload = CubeTelemetryPayload {
                        battery,
                        orientation: gyro_processor.current_orientation(),
                        quaternion: [q.x, q.y, q.z, q.w],
                        timestamp_ms: ts,
                    };
                    let _ = app.emit("cube-telemetry", &payload);
                }
                GanEvent::Facelets(_) => {}
            }
        }

        let mut guard = shared_state.lock().await;
        guard.ble.connected = false;
    });

    Ok(())
}

#[tauri::command]
async fn run_demo_solve(
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<(), String> {
    {
        let mut guard = state.lock().await;
        guard.demo_mode = true;
    }

    let t_perm = parse_moves("R U R' U' R' F R2 U' R' U' R U R' F'").map_err(|e| e.to_string())?;
    let sune = parse_moves("R U R' U R U2 R'").map_err(|e| e.to_string())?;
    let sune_inv = parse_moves("R U2 R' U' R U' R'").map_err(|e| e.to_string())?;
    let displace = parse_moves("R U R' U'").map_err(|e| e.to_string())?;
    let insert = parse_moves("y' U R U' R'").map_err(|e| e.to_string())?;
    let cross_scramble = parse_moves("D'").map_err(|e| e.to_string())?;
    let cross_solve = parse_moves("D").map_err(|e| e.to_string())?;

    let mut scramble_sequence = Vec::new();
    scramble_sequence.extend_from_slice(&t_perm);
    scramble_sequence.extend_from_slice(&sune_inv);
    for _ in 0..4 {
        scramble_sequence.extend_from_slice(&displace);
        scramble_sequence.push(Move::Y);
    }
    scramble_sequence.extend_from_slice(&cross_scramble);

    let mut tracker = CfopTracker::new();
    tracker.apply_scramble(&scramble_sequence);
    tracker.start_inspection(1000);
    tracker.start_solve(2000);

    let mut orientation = CubeOrientation::IDENTITY;
    let mut current_time_ms: u64 = 2000;

    let mut steps: Vec<(Move, u64)> = Vec::new();
    for m in cross_solve {
        steps.push((m, 120));
    }
    for (pair_idx, pair_moves) in [&insert, &insert, &insert, &insert].into_iter().enumerate() {
        for (move_idx, &m) in pair_moves.iter().enumerate() {
            let delay = if pair_idx == 2 && move_idx == 0 {
                420
            } else {
                120
            };
            steps.push((m, delay));
        }
    }
    for (i, &m) in sune.iter().enumerate() {
        steps.push((m, if i == 0 { 240 } else { 120 }));
    }
    for (i, &m) in t_perm.iter().enumerate() {
        steps.push((m, if i == 0 { 240 } else { 120 }));
    }

    for (m, delay_ms) in steps {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        current_time_ms += delay_ms;
        tracker.on_move(m, current_time_ms);

        if m.is_rotation() {
            orientation = orientation.apply_rotation(m);
            let telemetry = CubeTelemetryPayload {
                battery: Some(100),
                orientation,
                quaternion: [0.0, 0.0, 0.0, 1.0],
                timestamp_ms: current_time_ms,
            };
            let _ = app.emit("cube-telemetry", &telemetry);
        }

        let payload = CubeMovePayload {
            cube_move: m,
            move_notation: m.to_string(),
            stickers: tracker.cube().to_stickers().to_vec(),
            phase: tracker.current_phase(),
            timestamp_ms: current_time_ms,
        };
        let _ = app.emit("cube-move", &payload);
    }

    let analyzer = SolveAnalyzer::new();
    let report = analyzer.analyze(&tracker).map_err(|e| e.to_string())?;

    let next = {
        let mut guard = state.lock().await;
        guard.active_solves_mut().push(report.clone());
        guard.advance_to_new_scramble()
    };

    app.emit("solve-completed", &report)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("scramble-updated", &next);

    Ok(())
}

#[tauri::command]
async fn request_ai_coach(report: SolveReport) -> Result<String, String> {
    analyze_solve_with_gemini(&report)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn request_ai_session_coach(solves: Vec<SolveReport>) -> Result<String, String> {
    analyze_session_with_gemini(&solves)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn record_manual_solve(
    app: AppHandle,
    state: State<'_, SharedAppState>,
    total_time_ms: u64,
) -> Result<(), String> {
    let report = SolveReport {
        total_time_ms,
        active_time_ms: total_time_ms,
        total_moves_htm: 0,
        tps_raw: 0.0,
        tps_effective: 0.0,
        phases: Vec::new(),
        pauses: Vec::new(),
        rotation_tax: Default::default(),
        lockups: Vec::new(),
        anomalies: vec!["Manual".to_string()],
    };

    let next = {
        let mut guard = state.lock().await;
        guard.active_solves_mut().push(report.clone());
        guard.advance_to_new_scramble()
    };

    app.emit("solve-completed", &report)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("scramble-updated", &next);

    Ok(())
}

#[tauri::command]
async fn delete_solve(
    index: usize,
    state: State<'_, SharedAppState>,
) -> Result<(), String> {
    let mut guard = state.lock().await;
    let solves = guard.active_solves_mut();
    if index >= solves.len() {
        return Err(format!("Índice de solve fuera de rango: {index}"));
    }
    solves.remove(index);
    Ok(())
}

#[tauri::command]
async fn create_session(
    name: Option<String>,
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<String, String> {
    let final_name = {
        let mut guard = state.lock().await;
        let candidate = match name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) {
            Some(custom) => {
                if guard.sessions.iter().any(|s| s.name == custom) {
                    return Err(format!("Ya existe una serie llamada '{custom}'"));
                }
                custom
            }
            None => {
                let mut idx = guard.sessions.len() + 1;
                loop {
                    let auto_name = format!("Serie {idx}");
                    if !guard.sessions.iter().any(|s| s.name == auto_name) {
                        break auto_name;
                    }
                    idx += 1;
                }
            }
        };

        guard.sessions.push(SessionData {
            name: candidate.clone(),
            solves: Vec::new(),
        });
        guard.active_session = candidate.clone();
        candidate
    };

    let _ = app.emit("session-changed", &final_name);
    Ok(final_name)
}

#[tauri::command]
async fn switch_session(
    name: String,
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<Vec<SolveReport>, String> {
    let solves = {
        let mut guard = state.lock().await;
        let found = guard
            .sessions
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.solves.clone())
            .ok_or_else(|| format!("Serie no encontrada: '{name}'"))?;
        guard.active_session = name.clone();
        found
    };

    let _ = app.emit("session-changed", &name);
    Ok(solves)
}

#[tauri::command]
async fn clear_active_session(
    app: AppHandle,
    state: State<'_, SharedAppState>,
) -> Result<(), String> {
    let active_name = {
        let mut guard = state.lock().await;
        guard.active_solves_mut().clear();
        guard.active_session.clone()
    };

    let _ = app.emit("session-changed", &active_name);
    Ok(())
}

#[tauri::command]
async fn export_session(solves: Vec<SolveReport>) -> Result<String, String> {
    let filename = format!("sesion_{}.xlsx", Local::now().format("%Y%m%d_%H%M%S"));
    let path_for_task = filename.clone();

    tokio::task::spawn_blocking(move || export_session_to_excel(&path_for_task, &solves))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    Ok(filename)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> anyhow::Result<()> {
    let demo_mode = std::env::args().any(|arg| arg == "--demo");
    let first_moves = generate_wca_scramble(20);
    let initial_scramble = ScrambleState::from_moves(first_moves.clone(), false, false);
    let initial_state: SharedAppState = Arc::new(Mutex::new(AppState {
        ble: BleConnectionState::default(),
        sessions: vec![SessionData {
            name: "Serie 1".to_string(),
            solves: Vec::new(),
        }],
        active_session: "Serie 1".to_string(),
        demo_mode,
        scramble: initial_scramble,
        scramble_history: vec![first_moves],
        scramble_index: 0,
    }));

    tauri::Builder::default()
        .manage(initial_state)
        .invoke_handler(tauri::generate_handler![
            ping_backend,
            get_current_scramble,
            next_scramble,
            prev_scramble,
            start_ble_scan,
            run_demo_solve,
            request_ai_coach,
            request_ai_session_coach,
            record_manual_solve,
            delete_solve,
            create_session,
            switch_session,
            clear_active_session,
            export_session
        ])
        .run(tauri::generate_context!())
        .context("Error al ejecutar la aplicación Tauri")?;

    Ok(())
}
