use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use chrono::Local;
use cubo_cfop::analytics::SolveAnalyzer;
use cubo_cfop::ble::{scan_for_gan_cubes, AlignedTimestamp, GanBleClient, GanEvent};
use cubo_cfop::cfop::{CfopPhase, CfopTracker};
use cubo_cfop::cube::{parse_moves, Color, Move};
use cubo_cfop::export::export_session_to_excel;
use cubo_cfop::gyro::{CubeOrientation, GyroProcessor};
use cubo_cfop::{analyze_solve_with_gemini, Cube, SolveReport};
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
pub struct AppState {
    pub ble: BleConnectionState,
    pub session_history: Vec<SolveReport>,
    pub demo_mode: bool,
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

        while let Some((event, aligned)) = rx.recv().await {
            match event {
                GanEvent::Move(m_event) => {
                    let m = m_event.to_cube_move();
                    let ts = aligned.elapsed_solve_ms;
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
                            {
                                let mut guard = shared_state.lock().await;
                                guard.session_history.push(report.clone());
                            }
                            let _ = app.emit("solve-completed", &report);
                        }
                        tracker = CfopTracker::new();
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

    {
        let mut guard = state.lock().await;
        guard.session_history.push(report.clone());
    }

    app.emit("solve-completed", &report)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn request_ai_coach(report: SolveReport) -> Result<String, String> {
    analyze_solve_with_gemini(&report)
        .await
        .map_err(|e| e.to_string())
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
    let initial_state: SharedAppState = Arc::new(Mutex::new(AppState {
        ble: BleConnectionState::default(),
        session_history: Vec::new(),
        demo_mode,
    }));

    tauri::Builder::default()
        .manage(initial_state)
        .invoke_handler(tauri::generate_handler![
            ping_backend,
            start_ble_scan,
            run_demo_solve,
            request_ai_coach,
            export_session
        ])
        .run(tauri::generate_context!())
        .context("Error al ejecutar la aplicación Tauri")?;

    Ok(())
}
