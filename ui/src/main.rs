mod components;
mod models;
mod tauri_bridge;

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use components::{
    CenterTelemetryPanel, HistoryView, LeftSessionPanel, RightCoachPanel, TopBar,
};
use models::{
    default_solved_stickers, AppTheme, BleUiState, CoachMode, CubeMovePayload,
    CubeTelemetryPayload, ScrambleStateDto, SolveEntry, SolveReportDto, TimerState, UiTab,
};
use tauri_bridge::{
    invoke_clear_active_session, invoke_create_session, invoke_delete_solve,
    invoke_export_session, invoke_get_current_scramble, invoke_next_scramble,
    invoke_prev_scramble, invoke_record_manual_solve, invoke_request_ai_coach,
    invoke_request_ai_session_coach, invoke_run_demo_solve, invoke_start_ble_scan,
    invoke_switch_session, listen_event,
};

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

fn load_saved_theme() -> AppTheme {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("cubo_theme").ok().flatten())
        .map(|val| AppTheme::from_str(&val))
        .unwrap_or_default()
}

fn apply_theme(theme: AppTheme) {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Some(root) = doc.document_element() {
                let _ = root.set_attribute("data-theme", theme.as_str());
            }
        }
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.set_item("cubo_theme", theme.as_str());
        }
    }
}

#[component]
fn App() -> impl IntoView {
    let (active_tab, set_active_tab) = signal(UiTab::Dashboard);
    let (theme, set_theme) = signal(load_saved_theme());
    Effect::new(move |_| {
        apply_theme(theme.get());
    });
    let (ble, set_ble) = signal(BleUiState::default());
    let (scramble, set_scramble) = signal(ScrambleStateDto::default());
    let (elapsed_ms, set_elapsed_ms) = signal(0_u64);
    let (is_running, set_is_running) = signal(false);
    let (current_phase, set_current_phase) = signal("Scramble".to_string());
    let (last_move, set_last_move) = signal("-".to_string());
    let (stickers, set_stickers) = signal(default_solved_stickers());

    let (solves, set_solves) = signal(Vec::<SolveEntry>::new());
    let (sessions, set_sessions) = signal(vec!["Serie 1".to_string()]);
    let (active_session, set_active_session) = signal("Serie 1".to_string());
    let (selected_ao_sizes, set_selected_ao_sizes) = signal(vec![5_usize, 12_usize]);
    let (primary_ao_size, set_primary_ao_size) = signal(5_usize);
    let (selected_solve_id, set_selected_solve_id) = signal(Option::<usize>::None);
    let (coach_mode, set_coach_mode) = signal(CoachMode::Solve);
    let (coach_text, set_coach_text) = signal(Option::<String>::None);
    let (coach_loading, set_coach_loading) = signal(false);
    let (is_busy, set_is_busy) = signal(false);
    let (status_banner, set_status_banner) = signal(Option::<String>::None);

    let (timer_state, set_timer_state) = signal(TimerState::Idle);
    let (hold_start_perf, set_hold_start_perf) = signal(Option::<f64>::None);
    let (timer_start_perf, set_timer_start_perf) = signal(Option::<f64>::None);
    let last_event_perf = Rc::new(Cell::new(Option::<f64>::None));

    let stop_manual_timer = move || {
        if timer_state.get_untracked() != TimerState::Running {
            return;
        }
        let final_ms = timer_start_perf
            .get_untracked()
            .map(|start| (now_ms() - start).max(0.0) as u64)
            .unwrap_or_else(|| elapsed_ms.get_untracked());

        set_elapsed_ms.set(final_ms);
        set_is_running.set(false);
        set_timer_state.set(TimerState::Stopped);
        set_timer_start_perf.set(None);
        set_hold_start_perf.set(None);

        leptos::task::spawn_local(async move {
            if let Err(err) = invoke_record_manual_solve(final_ms).await {
                set_status_banner.set(Some(format!("Error registrando solve manual: {err}")));
            }
        });
    };

    // Cargar scramble inicial generado en backend al arrancar
    {
        leptos::task::spawn_local(async move {
            if let Ok(initial) = invoke_get_current_scramble().await {
                set_scramble.set(initial);
            }
        });
    }

    // Actualizador del cronómetro (~60fps) y transición Holding -> Ready (>= 300ms)
    {
        if let Some(win) = web_sys::window() {
            let cb = Closure::wrap(Box::new(move || {
                match timer_state.get_untracked() {
                    TimerState::Holding => {
                        if let Some(hold_start) = hold_start_perf.get_untracked() {
                            if now_ms() - hold_start >= 300.0 {
                                set_timer_state.set(TimerState::Ready);
                            }
                        }
                    }
                    TimerState::Running => {
                        if let Some(start) = timer_start_perf.get_untracked() {
                            let diff = (now_ms() - start).max(0.0) as u64;
                            set_elapsed_ms.set(diff);
                        }
                    }
                    _ => {}
                }
            }) as Box<dyn FnMut()>);

            let _ = win.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                16,
            );
            cb.forget();
        }
    }

    // Listeners globales de teclado para modo Stackmat (Barra Espaciadora)
    {
        if let Some(win) = web_sys::window() {
            let on_keydown = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                let state = timer_state.get_untracked();
                if state == TimerState::Running {
                    e.prevent_default();
                    stop_manual_timer();
                    return;
                }

                if e.code() == "Space" {
                    e.prevent_default();
                    if e.repeat() {
                        return;
                    }
                    if matches!(state, TimerState::Idle | TimerState::Stopped) {
                        set_hold_start_perf.set(Some(now_ms()));
                        set_timer_state.set(TimerState::Holding);
                    }
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            let on_keyup = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
                if e.code() != "Space" {
                    return;
                }
                e.prevent_default();

                match timer_state.get_untracked() {
                    TimerState::Holding => {
                        let held_enough = hold_start_perf
                            .get_untracked()
                            .map(|h| now_ms() - h >= 300.0)
                            .unwrap_or(false);
                        set_hold_start_perf.set(None);
                        if held_enough {
                            set_timer_start_perf.set(Some(now_ms()));
                            set_elapsed_ms.set(0);
                            set_is_running.set(true);
                            set_current_phase.set("Cross".to_string());
                            set_timer_state.set(TimerState::Running);
                        } else {
                            set_timer_state.set(TimerState::Idle);
                        }
                    }
                    TimerState::Ready => {
                        set_hold_start_perf.set(None);
                        set_timer_start_perf.set(Some(now_ms()));
                        set_elapsed_ms.set(0);
                        set_is_running.set(true);
                        set_current_phase.set("Cross".to_string());
                        set_timer_state.set(TimerState::Running);
                    }
                    TimerState::Stopped => {
                        set_timer_state.set(TimerState::Idle);
                    }
                    _ => {}
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            let target: &web_sys::EventTarget = win.as_ref();
            let _ = target.add_event_listener_with_callback(
                "keydown",
                on_keydown.as_ref().unchecked_ref(),
            );
            let _ = target.add_event_listener_with_callback(
                "keyup",
                on_keyup.as_ref().unchecked_ref(),
            );
            on_keydown.forget();
            on_keyup.forget();
        }
    }

    // Suscripción a `scramble-updated`
    {
        leptos::task::spawn_local(async move {
            let _ = listen_event::<ScrambleStateDto, _>("scramble-updated", move |payload| {
                set_scramble.set(payload);
            })
            .await;
        });
    }

    // Suscripción a eventos de Tauri (`cube-move`, `cube-telemetry`, `solve-completed`)
    {
        let last_perf_move = Rc::clone(&last_event_perf);
        leptos::task::spawn_local(async move {
            let _ = listen_event::<CubeMovePayload, _>("cube-move", move |payload| {
                let current_perf = now_ms();
                if let Some(prev) = last_perf_move.get() {
                    let delta = (current_perf - prev).max(0.0) as u64;
                    if delta < 5000 {
                        set_ble.update(|b| b.latency_ms = Some(delta.min(999)));
                    }
                }
                last_perf_move.set(Some(current_perf));

                let is_solving_move =
                    payload.phase != "Scramble" && payload.phase != "Inspection" && payload.phase != "Solved";

                if !is_running.get_untracked() && is_solving_move {
                    set_timer_start_perf.set(Some(current_perf));
                    set_elapsed_ms.set(0);
                    set_is_running.set(true);
                    set_timer_state.set(TimerState::Running);
                }

                if payload.stickers.len() == 54 {
                    set_stickers.set(payload.stickers);
                }
                set_last_move.set(payload.move_notation);
                set_current_phase.set(payload.phase.clone());

                if payload.phase == "Solved" {
                    set_is_running.set(false);
                    set_timer_start_perf.set(None);
                    set_timer_state.set(TimerState::Stopped);
                }
            })
            .await;
        });
    }

    {
        leptos::task::spawn_local(async move {
            let _ = listen_event::<CubeTelemetryPayload, _>("cube-telemetry", move |payload| {
                set_ble.update(|b| {
                    b.connected = true;
                    if payload.battery.is_some() {
                        b.battery = payload.battery;
                    }
                });
            })
            .await;
        });
    }

    {
        leptos::task::spawn_local(async move {
            let _ = listen_event::<serde_json::Value, _>("solve-completed", move |raw_json| {
                set_is_running.set(false);
                set_timer_start_perf.set(None);
                set_timer_state.set(TimerState::Stopped);
                set_current_phase.set("Solved".to_string());

                if let Ok(parsed) = serde_json::from_value::<SolveReportDto>(raw_json.clone()) {
                    set_elapsed_ms.set(parsed.total_time_ms);
                    let is_manual = parsed.phases.is_empty()
                        && parsed.anomalies.iter().any(|a| a == "Manual");
                    let scramble_str = scramble.get_untracked().moves.join(" ");
                    let mut new_id = 1;
                    set_solves.update(|list| {
                        new_id = list.len() + 1;
                        list.push(SolveEntry {
                            id: new_id,
                            scramble: scramble_str,
                            report: parsed,
                            raw_json: raw_json.clone(),
                        });
                    });
                    set_selected_solve_id.set(Some(new_id));

                    if is_manual {
                        if coach_mode.get_untracked() == CoachMode::Solve {
                            set_coach_text.set(Some(
                                "Solve manual registrado. Cambia a **Sesión Completa** para incluirlo en el análisis global del AI Coach.".to_string(),
                            ));
                        }
                    } else {
                        // Consultar automáticamente a Gemini AI Coach al finalizar el solve BLE/Demo
                        set_coach_mode.set(CoachMode::Solve);
                        set_coach_loading.set(true);
                        leptos::task::spawn_local(async move {
                            match invoke_request_ai_coach(&raw_json).await {
                                Ok(diag) => set_coach_text.set(Some(diag)),
                                Err(err) => set_coach_text.set(Some(format!(
                                    "No se pudo obtener diagnóstico de Gemini: {err}"
                                ))),
                            }
                            set_coach_loading.set(false);
                        });
                    }
                }
            })
            .await;
        });
    }

    let active_entry = Signal::derive(move || {
        let list = solves.get();
        match selected_solve_id.get() {
            Some(id) => list.iter().find(|s| s.id == id).cloned(),
            None => list.last().cloned(),
        }
    });

    let active_report = Signal::derive(move || active_entry.get().map(|e| e.report));
    let has_active_report = Signal::derive(move || active_entry.get().is_some());
    let has_session_solves = Signal::derive(move || !solves.get().is_empty());

    let run_coach_analysis = move |mode: CoachMode| match mode {
        CoachMode::Solve => {
            let Some(entry) = active_entry.get_untracked() else {
                return;
            };
            set_coach_loading.set(true);
            leptos::task::spawn_local(async move {
                match invoke_request_ai_coach(&entry.raw_json).await {
                    Ok(diag) => set_coach_text.set(Some(diag)),
                    Err(err) => set_coach_text.set(Some(format!(
                        "No se pudo analizar el solve con Gemini: {err}"
                    ))),
                }
                set_coach_loading.set(false);
            });
        }
        CoachMode::Session => {
            let raw_solves: Vec<serde_json::Value> =
                solves.get_untracked().into_iter().map(|s| s.raw_json).collect();
            if raw_solves.is_empty() {
                set_coach_text.set(Some(
                    "Registra al menos un solve para analizar la sesión completa.".to_string(),
                ));
                return;
            }
            set_coach_loading.set(true);
            leptos::task::spawn_local(async move {
                match invoke_request_ai_session_coach(&raw_solves).await {
                    Ok(diag) => set_coach_text.set(Some(diag)),
                    Err(err) => set_coach_text.set(Some(format!(
                        "No se pudo analizar la sesión con Gemini: {err}"
                    ))),
                }
                set_coach_loading.set(false);
            });
        }
    };

    let on_next_scramble = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            if let Err(err) = invoke_next_scramble().await {
                set_status_banner.set(Some(format!("Error generando scramble: {err}")));
            }
        });
    });

    let on_prev_scramble = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            if let Err(err) = invoke_prev_scramble().await {
                set_status_banner.set(Some(format!("Error cargando scramble anterior: {err}")));
            }
        });
    });

    let on_connect_ble = Callback::new(move |()| {
        set_status_banner.set(Some("Escaneando cubo GAN i4 por Bluetooth...".to_string()));
        leptos::task::spawn_local(async move {
            match invoke_start_ble_scan().await {
                Ok(()) => set_status_banner.set(Some(
                    "Escaneo BLE iniciado en segundo plano. Enciende tu GAN i4.".to_string(),
                )),
                Err(err) => set_status_banner.set(Some(format!("Error BLE: {err}"))),
            }
        });
    });

    let on_run_demo = Callback::new(move |()| {
        set_is_busy.set(true);
        set_is_running.set(false);
        set_timer_start_perf.set(None);
        set_elapsed_ms.set(0);
        set_coach_text.set(None);
        set_status_banner.set(Some(
            "Ejecutando simulación CFOP en tiempo real...".to_string(),
        ));

        leptos::task::spawn_local(async move {
            match invoke_run_demo_solve().await {
                Ok(()) => set_status_banner.set(Some("Simulación completada.".to_string())),
                Err(err) => set_status_banner.set(Some(format!("Error en simulación: {err}"))),
            }
            set_is_busy.set(false);
        });
    });

    let on_export_excel = Callback::new(move |()| {
        let raw_solves: Vec<serde_json::Value> =
            solves.get_untracked().into_iter().map(|s| s.raw_json).collect();
        if raw_solves.is_empty() {
            set_status_banner.set(Some("No hay solves para exportar a Excel.".to_string()));
            return;
        }
        set_is_busy.set(true);
        leptos::task::spawn_local(async move {
            match invoke_export_session(&raw_solves).await {
                Ok(path) => set_status_banner.set(Some(format!("Sesión exportada a: {path}"))),
                Err(err) => set_status_banner.set(Some(format!("Error exportando Excel: {err}"))),
            }
            set_is_busy.set(false);
        });
    });

    let on_change_coach_mode = Callback::new(move |mode: CoachMode| {
        set_coach_mode.set(mode);
        if mode == CoachMode::Session {
            run_coach_analysis(CoachMode::Session);
        } else if has_active_report.get_untracked() {
            run_coach_analysis(CoachMode::Solve);
        }
    });

    let on_reanalyze = Callback::new(move |()| {
        run_coach_analysis(coach_mode.get_untracked());
    });

    let on_select_from_history = Callback::new(move |id: usize| {
        set_selected_solve_id.set(Some(id));
        set_active_tab.set(UiTab::Dashboard);
    });

    let on_stopwatch_click = Callback::new(move |()| {
        stop_manual_timer();
    });

    let on_delete_solve = Callback::new(move |idx: usize| {
        leptos::task::spawn_local(async move {
            match invoke_delete_solve(idx).await {
                Ok(()) => {
                    let mut removed_id = None;
                    set_solves.update(|list| {
                        if idx < list.len() {
                            removed_id = Some(list[idx].id);
                            list.remove(idx);
                        }
                    });
                    let remaining = solves.get_untracked();
                    if remaining.is_empty() {
                        set_selected_solve_id.set(None);
                        set_elapsed_ms.set(0);
                        set_coach_text.set(None);
                    } else if selected_solve_id.get_untracked() == removed_id
                        || selected_solve_id.get_untracked().is_none()
                    {
                        set_selected_solve_id.set(remaining.last().map(|s| s.id));
                    }
                }
                Err(err) => {
                    set_status_banner.set(Some(format!("Error eliminando solve: {err}")));
                }
            }
        });
    });

    let on_delete_last = Callback::new(move |()| {
        let len = solves.get_untracked().len();
        if len > 0 {
            on_delete_solve.run(len - 1);
        }
    });

    let on_create_session = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            match invoke_create_session(None).await {
                Ok(new_name) => {
                    set_sessions.update(|list| {
                        if !list.contains(&new_name) {
                            list.push(new_name.clone());
                        }
                    });
                    set_active_session.set(new_name.clone());
                    set_solves.set(Vec::new());
                    set_selected_solve_id.set(None);
                    set_elapsed_ms.set(0);
                    set_coach_text.set(None);
                    set_status_banner.set(Some(format!("Nueva serie creada: {new_name}")));
                }
                Err(err) => set_status_banner.set(Some(format!("Error creando serie: {err}"))),
            }
        });
    });

    let on_switch_session = Callback::new(move |name: String| {
        leptos::task::spawn_local(async move {
            match invoke_switch_session(name.clone()).await {
                Ok(raw_list) => {
                    let mut restored = Vec::with_capacity(raw_list.len());
                    for (idx, raw_json) in raw_list.into_iter().enumerate() {
                        if let Ok(parsed) = serde_json::from_value::<SolveReportDto>(raw_json.clone())
                        {
                            restored.push(SolveEntry {
                                id: idx + 1,
                                scramble: String::new(),
                                report: parsed,
                                raw_json,
                            });
                        }
                    }
                    let last_id = restored.last().map(|s| s.id);
                    let last_time = restored.last().map(|s| s.report.total_time_ms).unwrap_or(0);
                    set_active_session.set(name);
                    set_solves.set(restored);
                    set_selected_solve_id.set(last_id);
                    set_elapsed_ms.set(last_time);
                    set_coach_text.set(None);
                }
                Err(err) => set_status_banner.set(Some(format!("Error cambiando de serie: {err}"))),
            }
        });
    });

    let on_clear_session = Callback::new(move |()| {
        leptos::task::spawn_local(async move {
            match invoke_clear_active_session().await {
                Ok(()) => {
                    set_solves.set(Vec::new());
                    set_selected_solve_id.set(None);
                    set_elapsed_ms.set(0);
                    set_coach_text.set(None);
                    set_status_banner.set(Some("Serie vaciada.".to_string()));
                }
                Err(err) => set_status_banner.set(Some(format!("Error vaciando serie: {err}"))),
            }
        });
    });

    let on_toggle_ao_size = Callback::new(move |n: usize| {
        set_selected_ao_sizes.update(|list| {
            if let Some(pos) = list.iter().position(|&x| x == n) {
                if list.len() > 1 {
                    list.remove(pos);
                }
            } else {
                list.push(n);
                list.sort_unstable();
                set_primary_ao_size.set(n);
            }
        });
        let current_list = selected_ao_sizes.get_untracked();
        if !current_list.contains(&primary_ao_size.get_untracked()) {
            if let Some(&first) = current_list.first() {
                set_primary_ao_size.set(first);
            }
        }
    });

    view! {
        <div class="app-shell">
            <TopBar
                ble=ble
                active_tab=active_tab
                set_active_tab=set_active_tab
                theme=theme
                set_theme=set_theme
                is_busy=is_busy
                on_connect_ble=on_connect_ble
                on_run_demo=on_run_demo
                on_export_excel=on_export_excel
            />

            <Show when=move || status_banner.get().is_some()>
                <div class="status-banner">
                    {move || status_banner.get().unwrap_or_default()}
                </div>
            </Show>

            <Show
                when=move || active_tab.get() == UiTab::Dashboard
                fallback=move || view! {
                    <HistoryView solves=solves on_select_solve=on_select_from_history />
                }
            >
                <main class="dashboard-grid">
                    <LeftSessionPanel
                        solves=solves
                        selected_solve_id=selected_solve_id
                        set_selected_solve_id=set_selected_solve_id
                        on_delete_solve=on_delete_solve
                        sessions=sessions
                        active_session=active_session
                        on_create_session=on_create_session
                        on_switch_session=on_switch_session
                        on_clear_session=on_clear_session
                        selected_ao_sizes=selected_ao_sizes
                        primary_ao_size=primary_ao_size
                        on_toggle_ao_size=on_toggle_ao_size
                    />
                    <CenterTelemetryPanel
                        scramble=scramble
                        on_next_scramble=on_next_scramble
                        on_prev_scramble=on_prev_scramble
                        elapsed_ms=elapsed_ms
                        timer_state=timer_state
                        on_stopwatch_click=on_stopwatch_click
                        on_delete_last=on_delete_last
                        has_solves=has_session_solves
                        current_phase=current_phase
                        last_move=last_move
                        stickers=stickers
                        active_report=active_report
                        solves=solves
                        coach_mode=coach_mode
                    />
                    <RightCoachPanel
                        coach_text=coach_text
                        coach_loading=coach_loading
                        coach_mode=coach_mode
                        on_change_coach_mode=on_change_coach_mode
                        has_active_report=has_active_report
                        has_session_solves=has_session_solves
                        on_reanalyze=on_reanalyze
                    />
                </main>
            </Show>
        </div>
    }
}

fn main() {
    leptos::mount::mount_to_body(App);
}
