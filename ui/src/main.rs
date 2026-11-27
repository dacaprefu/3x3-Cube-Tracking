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
    default_solved_stickers, BleUiState, CubeMovePayload, CubeTelemetryPayload, SolveEntry,
    SolveReportDto, UiTab,
};
use tauri_bridge::{
    invoke_export_session, invoke_request_ai_coach, invoke_run_demo_solve, invoke_start_ble_scan,
    listen_event,
};

const DEFAULT_SCRAMBLE: &str = "R U R' U' R' F R2 U' R' U' R U R' F' R U2 R' U' R U' R' D'";

fn now_ms() -> f64 {
    web_sys::window()
        .and_then(|w| w.performance())
        .map(|p| p.now())
        .unwrap_or(0.0)
}

#[component]
fn App() -> impl IntoView {
    let (active_tab, set_active_tab) = signal(UiTab::Dashboard);
    let (ble, set_ble) = signal(BleUiState::default());
    let (scramble, _set_scramble) = signal(DEFAULT_SCRAMBLE.to_string());
    let (elapsed_ms, set_elapsed_ms) = signal(0_u64);
    let (is_running, set_is_running) = signal(false);
    let (current_phase, set_current_phase) = signal("Scramble".to_string());
    let (last_move, set_last_move) = signal("-".to_string());
    let (stickers, set_stickers) = signal(default_solved_stickers());

    let (solves, set_solves) = signal(Vec::<SolveEntry>::new());
    let (selected_solve_id, set_selected_solve_id) = signal(Option::<usize>::None);
    let (coach_text, set_coach_text) = signal(Option::<String>::None);
    let (coach_loading, set_coach_loading) = signal(false);
    let (is_busy, set_is_busy) = signal(false);
    let (status_banner, set_status_banner) = signal(Option::<String>::None);

    let (timer_start_perf, set_timer_start_perf) = signal(Option::<f64>::None);
    let last_event_perf = Rc::new(Cell::new(Option::<f64>::None));

    // Actualizador del cronómetro al milisegundo mientras el solve está activo
    {
        if let Some(win) = web_sys::window() {
            let cb = Closure::wrap(Box::new(move || {
                if is_running.get_untracked() {
                    if let Some(start) = timer_start_perf.get_untracked() {
                        let diff = (now_ms() - start).max(0.0) as u64;
                        set_elapsed_ms.set(diff);
                    }
                }
            }) as Box<dyn FnMut()>);

            let _ = win.set_interval_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                16,
            );
            cb.forget();
        }
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

                if !is_running.get_untracked() && payload.phase != "Solved" {
                    set_timer_start_perf.set(Some(current_perf));
                    set_elapsed_ms.set(0);
                    set_is_running.set(true);
                }

                if payload.stickers.len() == 54 {
                    set_stickers.set(payload.stickers);
                }
                set_last_move.set(payload.move_notation);
                set_current_phase.set(payload.phase.clone());

                if payload.phase == "Solved" {
                    set_is_running.set(false);
                    set_timer_start_perf.set(None);
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
                set_current_phase.set("Solved".to_string());

                if let Ok(parsed) = serde_json::from_value::<SolveReportDto>(raw_json.clone()) {
                    set_elapsed_ms.set(parsed.total_time_ms);
                    let mut new_id = 1;
                    set_solves.update(|list| {
                        new_id = list.len() + 1;
                        list.push(SolveEntry {
                            id: new_id,
                            scramble: scramble.get_untracked(),
                            report: parsed,
                            raw_json: raw_json.clone(),
                        });
                    });
                    set_selected_solve_id.set(Some(new_id));

                    // Consultar automáticamente a Gemini AI Coach al finalizar el solve
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

    let on_reanalyze = Callback::new(move |()| {
        let Some(entry) = active_entry.get_untracked() else {
            return;
        };
        set_coach_loading.set(true);
        leptos::task::spawn_local(async move {
            match invoke_request_ai_coach(&entry.raw_json).await {
                Ok(diag) => set_coach_text.set(Some(diag)),
                Err(err) => set_coach_text.set(Some(format!(
                    "No se pudo re-analizar con Gemini: {err}"
                ))),
            }
            set_coach_loading.set(false);
        });
    });

    let on_select_from_history = Callback::new(move |id: usize| {
        set_selected_solve_id.set(Some(id));
        set_active_tab.set(UiTab::Dashboard);
    });

    view! {
        <div class="app-shell">
            <TopBar
                ble=ble
                active_tab=active_tab
                set_active_tab=set_active_tab
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
                    />
                    <CenterTelemetryPanel
                        scramble=scramble
                        elapsed_ms=elapsed_ms
                        is_running=is_running
                        current_phase=current_phase
                        last_move=last_move
                        stickers=stickers
                        active_report=active_report
                    />
                    <RightCoachPanel
                        coach_text=coach_text
                        coach_loading=coach_loading
                        has_active_report=has_active_report
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
