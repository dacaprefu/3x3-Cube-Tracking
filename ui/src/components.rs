use leptos::prelude::*;

use crate::models::{
    calc_pb, calc_trimmed_ao, format_ms_stopwatch, phase_label, sticker_hex, BleUiState,
    PhaseMetricsDto, SolveEntry, SolveReportDto, UiTab,
};

#[component]
pub fn TopBar(
    ble: ReadSignal<BleUiState>,
    active_tab: ReadSignal<UiTab>,
    set_active_tab: WriteSignal<UiTab>,
    is_busy: ReadSignal<bool>,
    on_connect_ble: Callback<()>,
    on_run_demo: Callback<()>,
    on_export_excel: Callback<()>,
) -> impl IntoView {
    view! {
        <header class="topbar">
            <div class="topbar-brand">
                <span class="brand-title">"CUBO CFOP // TELEMETRÍA"</span>
                <div class=move || {
                    if ble.get().connected {
                        "ble-badge connected"
                    } else {
                        "ble-badge disconnected"
                    }
                }>
                    <span class="status-dot"></span>
                    <span>
                        {move || {
                            if ble.get().connected {
                                "GAN i4 Conectado"
                            } else {
                                "GAN i4 Desconectado"
                            }
                        }}
                    </span>
                </div>
                <div class="telemetry-pills">
                    <span class="pill">
                        {move || match ble.get().battery {
                            Some(b) => format!("BAT: {b}%"),
                            None => "BAT: --%".to_string(),
                        }}
                    </span>
                    <span class="pill">
                        {move || match ble.get().latency_ms {
                            Some(lat) => format!("LAT: {lat} ms"),
                            None => "LAT: -- ms".to_string(),
                        }}
                    </span>
                </div>
            </div>

            <div class="topbar-tabs">
                <button
                    class=move || if active_tab.get() == UiTab::Dashboard { "tab-btn active" } else { "tab-btn" }
                    on:click=move |_| set_active_tab.set(UiTab::Dashboard)
                >
                    "Dashboard"
                </button>
                <button
                    class=move || if active_tab.get() == UiTab::History { "tab-btn active" } else { "tab-btn" }
                    on:click=move |_| set_active_tab.set(UiTab::History)
                >
                    "Historial"
                </button>
            </div>

            <div class="topbar-actions">
                <button class="btn" disabled=move || is_busy.get() on:click=move |_| on_connect_ble.run(())>
                    "Conectar GAN i4"
                </button>
                <button class="btn btn-primary" disabled=move || is_busy.get() on:click=move |_| on_run_demo.run(())>
                    "Simular Solve (Demo)"
                </button>
                <button class="btn btn-accent" disabled=move || is_busy.get() on:click=move |_| on_export_excel.run(())>
                    "Exportar Excel"
                </button>
            </div>
        </header>
    }
}

#[component]
pub fn LeftSessionPanel(
    solves: ReadSignal<Vec<SolveEntry>>,
    selected_solve_id: ReadSignal<Option<usize>>,
    set_selected_solve_id: WriteSignal<Option<usize>>,
) -> impl IntoView {
    let (list_expanded, set_list_expanded) = signal(true);

    let ao5_text = move || match calc_trimmed_ao(&solves.get(), 5) {
        Some(ms) => format_ms_stopwatch(ms),
        None => "--:--.---".to_string(),
    };
    let ao12_text = move || match calc_trimmed_ao(&solves.get(), 12) {
        Some(ms) => format_ms_stopwatch(ms),
        None => "--:--.---".to_string(),
    };
    let pb_text = move || match calc_pb(&solves.get()) {
        Some(ms) => format_ms_stopwatch(ms),
        None => "--:--.---".to_string(),
    };

    view! {
        <aside class="panel">
            <div class="panel-header">
                <span class="panel-title">"Sesión Actual"</span>
                <span class="solve-meta">{move || format!("{} solves", solves.get().len())}</span>
            </div>

            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-label">"Ao5"</div>
                    <div class="stat-value">{ao5_text}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">"Ao12"</div>
                    <div class="stat-value">{ao12_text}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">"Mejor (PB)"</div>
                    <div class="stat-value pb">{pb_text}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">"Total"</div>
                    <div class="stat-value">{move || solves.get().len().to_string()}</div>
                </div>
            </div>

            <div class="panel-header">
                <span class="panel-title">"Solves Recientes"</span>
                <button
                    class="btn"
                    style="padding: 3px 8px; font-size: 11px;"
                    on:click=move |_| set_list_expanded.update(|v| *v = !*v)
                >
                    {move || if list_expanded.get() { "Ocultar" } else { "Mostrar" }}
                </button>
            </div>

            <Show when=move || list_expanded.get()>
                <div class="solve-list">
                    {move || {
                        let items = solves.get();
                        if items.is_empty() {
                            return view! {
                                <div class="coach-placeholder">"Sin solves registrados aún."</div>
                            }
                            .into_any();
                        }

                        items
                            .into_iter()
                            .rev()
                            .map(|entry| {
                                let entry_id = entry.id;
                                let is_selected = move || selected_solve_id.get() == Some(entry_id);
                                let time_str = format_ms_stopwatch(entry.report.total_time_ms);
                                let tps_str = format!("{:.2} TPS", entry.report.tps_effective);
                                let anomalies = entry.report.anomalies.clone();

                                view! {
                                    <div
                                        class=move || if is_selected() { "solve-item selected" } else { "solve-item" }
                                        on:click=move |_| set_selected_solve_id.set(Some(entry_id))
                                    >
                                        <div class="solve-item-header">
                                            <span class="solve-time">{format!("#{}  {}", entry_id, time_str)}</span>
                                            <span class="solve-meta">{tps_str}</span>
                                        </div>
                                        {(!anomalies.is_empty()).then(|| {
                                            view! {
                                                <div class="anomaly-tags">
                                                    {anomalies
                                                        .into_iter()
                                                        .map(|tag| view! { <span class="anomaly-tag">{tag}</span> })
                                                        .collect_view()}
                                                </div>
                                            }
                                        })}
                                    </div>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>
            </Show>
        </aside>
    }
}

#[component]
fn CubeFaceGrid(stickers: ReadSignal<Vec<String>>, face_index: usize) -> impl IntoView {
    let start = face_index * 9;
    view! {
        <div class="cube-face">
            {(0..9)
                .map(|offset| {
                    let idx = start + offset;
                    view! {
                        <div
                            class="sticker"
                            style=move || {
                                let list = stickers.get();
                                let color_name = list.get(idx).map(String::as_str).unwrap_or("");
                                format!("background-color: {};", sticker_hex(color_name))
                            }
                        ></div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn CenterTelemetryPanel(
    scramble: ReadSignal<String>,
    elapsed_ms: ReadSignal<u64>,
    is_running: ReadSignal<bool>,
    current_phase: ReadSignal<String>,
    last_move: ReadSignal<String>,
    stickers: ReadSignal<Vec<String>>,
    active_report: Signal<Option<SolveReportDto>>,
) -> impl IntoView {
    view! {
        <section class="panel">
            <div class="scramble-box">
                <div class="scramble-label">"Scramble Actual"</div>
                <div class="scramble-text">{move || scramble.get()}</div>
            </div>

            <div class="timer-stage">
                <div class=move || {
                    if is_running.get() {
                        "stopwatch running"
                    } else if current_phase.get() == "Solved" {
                        "stopwatch solved"
                    } else {
                        "stopwatch"
                    }
                }>
                    {move || format_ms_stopwatch(elapsed_ms.get())}
                </div>
                <div style="display: flex; gap: 8px; align-items: center;">
                    <span class="phase-badge">
                        {move || format!("FASE: {}", phase_label(&current_phase.get()))}
                    </span>
                    <span class="pill">
                        {move || format!("ÚLTIMO GIRO: {}", last_move.get())}
                    </span>
                </div>
            </div>

            <div class="cube-net-wrapper">
                <div class="cube-net">
                    <div></div>
                    <CubeFaceGrid stickers=stickers face_index=0 />
                    <div></div>
                    <div></div>

                    <CubeFaceGrid stickers=stickers face_index=4 />
                    <CubeFaceGrid stickers=stickers face_index=2 />
                    <CubeFaceGrid stickers=stickers face_index=5 />
                    <CubeFaceGrid stickers=stickers face_index=3 />

                    <div></div>
                    <CubeFaceGrid stickers=stickers face_index=1 />
                    <div></div>
                    <div></div>
                </div>
            </div>

            <div class="cfop-section">
                <div class="panel-header" style="padding-bottom: 6px;">
                    <span class="panel-title">"Desglose CFOP"</span>
                    <span class="solve-meta">
                        {move || match active_report.get() {
                            Some(r) => format!(
                                "HTM: {} | TPS Bruto: {:.2} | TPS Efec: {:.2}",
                                r.total_moves_htm, r.tps_raw, r.tps_effective
                            ),
                            None => "Esperando finalización del solve...".to_string(),
                        }}
                    </span>
                </div>

                {move || match active_report.get() {
                    Some(report) => {
                        let total = report.total_time_ms.max(1) as f64;
                        report
                            .phases
                            .into_iter()
                            .map(|p: PhaseMetricsDto| {
                                let pct = ((p.duration_ms as f64 / total) * 100.0).clamp(0.0, 100.0);
                                let bar_class = if p.is_inefficient {
                                    "cfop-bar-fill inefficient"
                                } else {
                                    "cfop-bar-fill"
                                };
                                view! {
                                    <div class="cfop-row">
                                        <span class="cfop-name">{phase_label(&p.phase).to_string()}</span>
                                        <div class="cfop-bar-track">
                                            <div class=bar_class style=format!("width: {pct:.1}%;")></div>
                                        </div>
                                        <span class="cfop-metrics">
                                            {format!("{}ms ({:.0}%) | {:.1} TPS", p.duration_ms, pct, p.tps_effective)}
                                        </span>
                                    </div>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }
                    None => view! {
                        <div class="coach-placeholder">
                            "Completa un solve o ejecuta la simulación demo para visualizar el desglose CFOP."
                        </div>
                    }
                    .into_any(),
                }}
            </div>
        </section>
    }
}

#[component]
pub fn RightCoachPanel(
    coach_text: ReadSignal<Option<String>>,
    coach_loading: ReadSignal<bool>,
    has_active_report: Signal<bool>,
    on_reanalyze: Callback<()>,
) -> impl IntoView {
    view! {
        <aside class="panel">
            <div class="panel-header">
                <span class="panel-title">"AI Coach Insights"</span>
                <button
                    class="btn"
                    style="padding: 4px 10px; font-size: 11px;"
                    disabled=move || coach_loading.get() || !has_active_report.get()
                    on:click=move |_| on_reanalyze.run(())
                >
                    {move || if coach_loading.get() { "Analizando..." } else { "Re-analizar" }}
                </button>
            </div>

            <div class="coach-card">
                <div class="coach-content">
                    {move || {
                        if coach_loading.get() {
                            return view! {
                                <div class="coach-placeholder">
                                    "Consultando a Gemini AI Coach sobre pérdidas de tiempo, par F2L más lento, fluidez y ejercicio recomendado..."
                                </div>
                            }
                            .into_any();
                        }

                        match coach_text.get() {
                            Some(text) => view! { <div>{text}</div> }.into_any(),
                            None => view! {
                                <div class="coach-placeholder">
                                    "Al terminar un solve, aquí aparecerá el diagnóstico automático de Gemini: mayor pérdida de tiempo, par F2L más lento, fluidez y ejercicio práctico."
                                </div>
                            }
                            .into_any(),
                        }
                    }}
                </div>
            </div>
        </aside>
    }
}

#[component]
pub fn HistoryView(
    solves: ReadSignal<Vec<SolveEntry>>,
    on_select_solve: Callback<usize>,
) -> impl IntoView {
    view! {
        <section class="history-panel">
            <div class="panel-header">
                <span class="panel-title">"Historial Completo de Sesión"</span>
                <span class="solve-meta">{move || format!("{} solves registrados", solves.get().len())}</span>
            </div>

            <Show
                when=move || !solves.get().is_empty()
                fallback=|| view! {
                    <p class="coach-placeholder" style="margin-top: 14px;">
                        "No hay solves en el historial de esta sesión todavía."
                    </p>
                }
            >
                <table class="history-table">
                    <thead>
                        <tr>
                            <th>"#"</th>
                            <th>"Tiempo"</th>
                            <th>"HTM"</th>
                            <th>"TPS Efec."</th>
                            <th>"Rot. F2L"</th>
                            <th>"Anomalías"</th>
                            <th>"Acción"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            solves
                                .get()
                                .into_iter()
                                .rev()
                                .map(|entry| {
                                    let id = entry.id;
                                    let time = format_ms_stopwatch(entry.report.total_time_ms);
                                    let htm = entry.report.total_moves_htm;
                                    let tps = format!("{:.2}", entry.report.tps_effective);
                                    let rots = entry.report.rotation_tax.total_y_rotations_f2l;
                                    let anom = if entry.report.anomalies.is_empty() {
                                        "Ninguna".to_string()
                                    } else {
                                        entry.report.anomalies.join(", ")
                                    };
                                    view! {
                                        <tr>
                                            <td>{id}</td>
                                            <td>{time}</td>
                                            <td>{htm}</td>
                                            <td>{tps}</td>
                                            <td>{rots}</td>
                                            <td>{anom}</td>
                                            <td>
                                                <button
                                                    class="btn"
                                                    style="padding: 4px 9px; font-size: 11px;"
                                                    on:click=move |_| on_select_solve.run(id)
                                                >
                                                    "Ver en Dashboard"
                                                </button>
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()
                        }}
                    </tbody>
                </table>
            </Show>
        </section>
    }
}
