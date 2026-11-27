use std::f64::consts::PI;

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use crate::models::{
    calc_ao_n, calc_best_ao_n, calc_pb, compute_radar_scores, compute_time_distribution,
    format_ms_stopwatch, markdown_to_safe_html, phase_label, sticker_hex, AppTheme, BleUiState,
    CfopRadarScores, CoachMode, DistributionStepMode, PhaseMetricsDto, ScrambleStateDto,
    SolveEntry, SolveReportDto, TimerState, UiTab, AVAILABLE_AO_SIZES,
};

#[component]
pub fn TopBar(
    ble: ReadSignal<BleUiState>,
    active_tab: ReadSignal<UiTab>,
    set_active_tab: WriteSignal<UiTab>,
    theme: ReadSignal<AppTheme>,
    set_theme: WriteSignal<AppTheme>,
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

            <div class="topbar-nav-group">
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

                <div class="theme-selector" title="Tema visual">
                    {[AppTheme::DarkBlue, AppTheme::Gray, AppTheme::Light]
                        .into_iter()
                        .map(|t| {
                            view! {
                                <button
                                    class=move || if theme.get() == t { "theme-pill active" } else { "theme-pill" }
                                    on:click=move |_| set_theme.set(t)
                                >
                                    {t.label()}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
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
    on_delete_solve: Callback<usize>,
    sessions: ReadSignal<Vec<String>>,
    active_session: ReadSignal<String>,
    on_create_session: Callback<()>,
    on_switch_session: Callback<String>,
    on_clear_session: Callback<()>,
    selected_ao_sizes: ReadSignal<Vec<usize>>,
    primary_ao_size: ReadSignal<usize>,
    on_toggle_ao_size: Callback<usize>,
) -> impl IntoView {
    let (list_expanded, set_list_expanded) = signal(true);
    let (confirm_clear, set_confirm_clear) = signal(false);
    let (dist_mode, set_dist_mode) = signal(DistributionStepMode::Auto);

    let pb_text = move || match calc_pb(&solves.get()) {
        Some(ms) => format_ms_stopwatch(ms),
        None => "--:--.---".to_string(),
    };

    let best_ao_card_text = move || {
        let n = primary_ao_size.get();
        match calc_best_ao_n(&solves.get(), n) {
            Some(ms) => format_ms_stopwatch(ms),
            None => "--:--.---".to_string(),
        }
    };

    let on_clear_click = move |_| {
        if confirm_clear.get_untracked() {
            set_confirm_clear.set(false);
            on_clear_session.run(());
        } else {
            set_confirm_clear.set(true);
            if let Some(win) = web_sys::window() {
                let cb = Closure::once(move || {
                    set_confirm_clear.set(false);
                });
                let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    3000,
                );
                cb.forget();
            }
        }
    };

    view! {
        <aside class="panel">
            <div class="panel-header">
                <span class="panel-title">"Sesión Actual"</span>
                <span class="solve-meta">{move || format!("{} solves", solves.get().len())}</span>
            </div>

            <div class="session-controls">
                <select
                    class="session-select"
                    prop:value=move || active_session.get()
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        on_switch_session.run(val);
                    }
                >
                    {move || {
                        let current = active_session.get();
                        sessions
                            .get()
                            .into_iter()
                            .map(|name| {
                                let is_sel = name == current;
                                let opt_val = name.clone();
                                view! {
                                    <option value=opt_val selected=is_sel>
                                        {name}
                                    </option>
                                }
                            })
                            .collect_view()
                    }}
                </select>
                <button
                    class="btn"
                    style="padding: 4px 8px; font-size: 11px;"
                    on:click=move |_| on_create_session.run(())
                >
                    "+ Nueva Serie"
                </button>
                <button
                    class=move || {
                        if confirm_clear.get() {
                            "btn btn-danger-confirm"
                        } else {
                            "btn btn-danger-subtle"
                        }
                    }
                    style="padding: 4px 8px; font-size: 11px;"
                    disabled=move || solves.get().is_empty()
                    on:click=on_clear_click
                >
                    {move || if confirm_clear.get() { "¿Confirmar?" } else { "Vaciar Serie" }}
                </button>
            </div>

            <div class="ao-selector">
                <span class="stat-label" style="margin-right: 2px;">"Ao:"</span>
                {AVAILABLE_AO_SIZES
                    .into_iter()
                    .map(|n| {
                        let is_active = move || selected_ao_sizes.get().contains(&n);
                        view! {
                            <button
                                class=move || if is_active() { "ao-pill active" } else { "ao-pill" }
                                on:click=move |_| on_toggle_ao_size.run(n)
                            >
                                {n.to_string()}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>

            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-label">"Mejor (PB)"</div>
                    <div class="stat-value pb">{pb_text}</div>
                </div>
                <div class="stat-card">
                    <div class="stat-label">"Total"</div>
                    <div class="stat-value">{move || solves.get().len().to_string()}</div>
                </div>

                <div class="stat-card highlight" style="grid-column: span 2;">
                    <div class="stat-label">
                        {move || format!("Mejor Ao{} de la Serie", primary_ao_size.get())}
                    </div>
                    <div class="stat-value pb">{best_ao_card_text}</div>
                </div>

                {move || {
                    let list = solves.get();
                    selected_ao_sizes
                        .get()
                        .into_iter()
                        .map(|n| {
                            let cur = calc_ao_n(&list, n)
                                .map(format_ms_stopwatch)
                                .unwrap_or_else(|| "--:--.---".to_string());
                            let best = calc_best_ao_n(&list, n)
                                .map(format_ms_stopwatch)
                                .unwrap_or_else(|| "--:--.---".to_string());
                            view! {
                                <div class="stat-card">
                                    <div class="stat-label">{format!("Ao{n}")}</div>
                                    <div class="stat-value">{cur}</div>
                                    <div class="stat-sub">{format!("Mejor: {best}")}</div>
                                </div>
                            }
                        })
                        .collect_view()
                }}
            </div>

            <div class="dist-card">
                <div class="dist-header">
                    <span class="panel-title">"Distribución de Tiempos"</span>
                    <div class="dist-step-selector" title="Intervalo de distribución">
                        {[
                            (DistributionStepMode::Auto, "Auto"),
                            (DistributionStepMode::HalfSec, "0.5s"),
                            (DistributionStepMode::OneSec, "1.0s"),
                            (DistributionStepMode::TwoSec, "2.0s"),
                        ]
                        .into_iter()
                        .map(|(mode, label)| {
                            view! {
                                <button
                                    class=move || if dist_mode.get() == mode { "ao-pill active" } else { "ao-pill" }
                                    on:click=move |_| set_dist_mode.set(mode)
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                    </div>
                </div>
                <div class="dist-meta-row">
                    <span class="solve-meta" style="font-family: var(--font-mono);">
                        {move || {
                            let dist = compute_time_distribution(&solves.get(), dist_mode.get());
                            match (dist.mean_ms, dist.std_dev_ms) {
                                (Some(m), Some(sd)) => format!(
                                    "Mean: {:.2}s | σ: {:.2}s",
                                    m as f64 / 1000.0,
                                    sd as f64 / 1000.0
                                ),
                                _ => "Mean: -- | σ: --".to_string(),
                            }
                        }}
                    </span>
                </div>

                {move || {
                    let dist = compute_time_distribution(&solves.get(), dist_mode.get());
                    if dist.buckets.is_empty() {
                        return view! {
                            <div class="coach-placeholder">
                                "Registra solves para ver la distribución de tiempos"
                            </div>
                        }
                        .into_any();
                    }

                    let max_c = dist.max_count.max(1) as f64;
                    view! {
                        <div class="dist-list">
                            {dist
                                .buckets
                                .into_iter()
                                .map(|b| {
                                    let width_pct = (b.count as f64 / max_c) * 100.0;
                                    let bar_class = if b.has_pb {
                                        "dist-bar-fill pb"
                                    } else if b.is_mode {
                                        "dist-bar-fill mode"
                                    } else {
                                        "dist-bar-fill"
                                    };
                                    view! {
                                        <div class="dist-row" title=b.range_tooltip>
                                            <span class="dist-label">{b.label}</span>
                                            <div class="dist-bar-track">
                                                <div class=bar_class style=format!("width: {width_pct:.1}%;")></div>
                                            </div>
                                            <span class="dist-count">
                                                {format!("{} ({:.0}%)", b.count, b.percent)}
                                            </span>
                                            <span class="dist-cum">
                                                <span class="dist-cum-label">{b.cum_label}</span>
                                                <span class="dist-cum-frac">
                                                    {format!("{}/{}", b.cum_count, b.total_count)}
                                                </span>
                                            </span>
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any()
                }}
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
                            .enumerate()
                            .rev()
                            .map(|(idx, entry)| {
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
                                            <span class="solve-time">{format!("#{}  {}", idx + 1, time_str)}</span>
                                            <div style="display: flex; align-items: center; gap: 6px;">
                                                <span class="solve-meta">{tps_str}</span>
                                                <button
                                                    class="solve-delete-btn"
                                                    title="Eliminar solve"
                                                    on:click=move |e| {
                                                        e.stop_propagation();
                                                        on_delete_solve.run(idx);
                                                    }
                                                >
                                                    "×"
                                                </button>
                                            </div>
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
fn CfopRadarChart(scores: Signal<CfopRadarScores>, mode_label: Signal<String>) -> impl IntoView {
    let cx = 140.0_f64;
    let cy = 128.0_f64;
    let max_r = 78.0_f64;
    let labels = ["Cruz", "F2L", "OLL", "PLL", "Lookahead", "Fluidez"];

    let polar = move |idx: usize, ratio: f64| -> (f64, f64) {
        let angle = -PI / 2.0 + (idx as f64) * (2.0 * PI / 6.0);
        let r = max_r * ratio;
        (cx + r * angle.cos(), cy + r * angle.sin())
    };

    let ring_points = move |ratio: f64| -> String {
        (0..6)
            .map(|i| {
                let (x, y) = polar(i, ratio);
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let data_values = move || {
        let s = scores.get();
        [s.cross, s.f2l, s.oll, s.pll, s.lookahead, s.fluency]
    };

    let data_polygon_points = move || {
        let vals = data_values();
        vals.iter()
            .enumerate()
            .map(|(i, v)| {
                let (x, y) = polar(i, (v / 100.0).clamp(0.0, 1.0));
                format!("{x:.1},{y:.1}")
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    view! {
        <div class="radar-card">
            <div class="panel-header" style="padding-bottom: 6px;">
                <span class="panel-title">"Perfil CFOP"</span>
                <span class="solve-meta">{move || mode_label.get()}</span>
            </div>
            <svg viewBox="0 0 280 256" class="radar-svg">
                {[0.25, 0.5, 0.75, 1.0]
                    .into_iter()
                    .map(|ratio| {
                        view! {
                            <polygon
                                points=ring_points(ratio)
                                fill="none"
                                stroke="var(--border-color)"
                                stroke-width="1"
                            />
                        }
                    })
                    .collect_view()}

                {(0..6)
                    .map(|i| {
                        let (x, y) = polar(i, 1.0);
                        let (lx, ly) = polar(i, 1.24);
                        view! {
                            <line
                                x1=format!("{cx:.1}")
                                y1=format!("{cy:.1}")
                                x2=format!("{x:.1}")
                                y2=format!("{y:.1}")
                                stroke="var(--border-color)"
                                stroke-width="1"
                            />
                            <text
                                x=format!("{lx:.1}")
                                y=format!("{ly:.1}")
                                fill="var(--text-secondary)"
                                font-size="10"
                                font-weight="600"
                                text-anchor="middle"
                                dominant-baseline="middle"
                            >
                                {labels[i]}
                            </text>
                        }
                    })
                    .collect_view()}

                <polygon
                    points=data_polygon_points
                    fill="var(--radar-fill)"
                    stroke="var(--accent-primary)"
                    stroke-width="2"
                />

                {move || {
                    let vals = data_values();
                    vals.into_iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let (x, y) = polar(i, (v / 100.0).clamp(0.0, 1.0));
                            view! {
                                <circle
                                    cx=format!("{x:.1}")
                                    cy=format!("{y:.1}")
                                    r="3.5"
                                    fill="var(--accent-primary)"
                                    stroke="var(--bg-app)"
                                    stroke-width="1.2"
                                >
                                    <title>{format!("{}: {:.0}/100", labels[i], v)}</title>
                                </circle>
                            }
                        })
                        .collect_view()
                }}
            </svg>
        </div>
    }
}

#[component]
pub fn CenterTelemetryPanel(
    scramble: ReadSignal<ScrambleStateDto>,
    on_next_scramble: Callback<()>,
    on_prev_scramble: Callback<()>,
    elapsed_ms: ReadSignal<u64>,
    timer_state: ReadSignal<TimerState>,
    on_stopwatch_click: Callback<()>,
    on_delete_last: Callback<()>,
    has_solves: Signal<bool>,
    current_phase: ReadSignal<String>,
    last_move: ReadSignal<String>,
    stickers: ReadSignal<Vec<String>>,
    active_report: Signal<Option<SolveReportDto>>,
    solves: ReadSignal<Vec<SolveEntry>>,
    coach_mode: ReadSignal<CoachMode>,
) -> impl IntoView {
    let radar_scores = Signal::derive(move || {
        if coach_mode.get() == CoachMode::Session {
            let list: Vec<SolveReportDto> = solves.get().into_iter().map(|s| s.report).collect();
            if !list.is_empty() {
                return compute_radar_scores(&list);
            }
        }
        match active_report.get() {
            Some(r) => compute_radar_scores(&[r]),
            None => CfopRadarScores::default(),
        }
    });

    let radar_mode_label = Signal::derive(move || {
        if coach_mode.get() == CoachMode::Session && !solves.get().is_empty() {
            "Promedio Sesión".to_string()
        } else if active_report.get().is_some() {
            "Solve Actual".to_string()
        } else {
            "Sin datos".to_string()
        }
    });

    view! {
        <section class="panel">
            <div class="scramble-box">
                <div class="scramble-header">
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span class="scramble-label" style="margin-bottom: 0;">"Scramble WCA (3x3)"</span>
                        <Show when=move || scramble.get().is_reviewing_previous>
                            <span class="scramble-review-tag">"[Repasando anterior]"</span>
                        </Show>
                    </div>
                    <div class="scramble-actions">
                        <button
                            class="btn"
                            style="padding: 3px 10px; font-size: 11px;"
                            disabled=move || !scramble.get().has_prev
                            on:click=move |_| on_prev_scramble.run(())
                        >
                            "← Anterior"
                        </button>
                        <button
                            class="btn"
                            style="padding: 3px 10px; font-size: 11px;"
                            on:click=move |_| on_next_scramble.run(())
                        >
                            {move || {
                                if scramble.get().is_reviewing_previous {
                                    "Siguiente →"
                                } else {
                                    "Nuevo Scramble"
                                }
                            }}
                        </button>
                    </div>
                </div>

                <div class="scramble-moves">
                    {move || {
                        let st = scramble.get();
                        if st.moves.is_empty() {
                            return view! { <span class="scramble-move pending">"Generando scramble..."</span> }.into_any();
                        }
                        st.moves
                            .into_iter()
                            .enumerate()
                            .map(|(idx, mv)| {
                                let class_name = if st.is_scrambled_ready || idx < st.completed_count {
                                    "scramble-move done"
                                } else if idx == st.completed_count {
                                    "scramble-move current"
                                } else {
                                    "scramble-move pending"
                                };
                                view! { <span class=class_name>{mv}</span> }
                            })
                            .collect_view()
                            .into_any()
                    }}
                </div>

                {move || {
                    let st = scramble.get();
                    if let Some(corr) = st.correction_move {
                        view! {
                            <div class="scramble-correction-alert">
                                {format!("Giro erróneo detectado — Corrige con: {corr}")}
                            </div>
                        }
                        .into_any()
                    } else if st.is_scrambled_ready {
                        view! {
                            <div class="scramble-ready-badge">
                                "Cubo mezclado — Listo para inspeccionar / resolver"
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }
                }}
            </div>

            <div class="timer-stage">
                <div
                    class=move || match timer_state.get() {
                        TimerState::Holding => "stopwatch holding",
                        TimerState::Ready => "stopwatch ready",
                        TimerState::Running => "stopwatch running",
                        TimerState::Stopped if current_phase.get() == "Solved" => "stopwatch solved",
                        TimerState::Idle | TimerState::Stopped => "stopwatch",
                    }
                    on:click=move |_| on_stopwatch_click.run(())
                    title="Mantén Espacio ≥300ms para iniciar; cualquier tecla o clic para detener"
                >
                    {move || format_ms_stopwatch(elapsed_ms.get())}
                </div>
                <div style="display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: center;">
                    <span class="phase-badge">
                        {move || match timer_state.get() {
                            TimerState::Holding => "STACKMAT: MANTÉN ESPACIO...".to_string(),
                            TimerState::Ready => "STACKMAT: ¡LISTO! SUELTA".to_string(),
                            TimerState::Running => format!("FASE: {}", phase_label(&current_phase.get())),
                            TimerState::Idle | TimerState::Stopped => {
                                format!("FASE: {}", phase_label(&current_phase.get()))
                            }
                        }}
                    </span>
                    <span class="pill">
                        {move || format!("ÚLTIMO GIRO: {}", last_move.get())}
                    </span>
                    <button
                        class="btn"
                        style="padding: 3px 9px; font-size: 11px;"
                        disabled=move || !has_solves.get() || timer_state.get() == TimerState::Running
                        on:click=move |_| on_delete_last.run(())
                    >
                        "Borrar último"
                    </button>
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

            <div class="cfop-analytics-split">
                <div class="cfop-section">
                    <div class="panel-header" style="padding-bottom: 6px;">
                        <span class="panel-title">"Desglose CFOP"</span>
                        <span class="solve-meta">
                            {move || match active_report.get() {
                                Some(r) => format!(
                                    "HTM: {} | TPS Bruto: {:.2} | TPS Efec: {:.2}",
                                    r.total_moves_htm, r.tps_raw, r.tps_effective
                                ),
                                None => "Esperando finalización...".to_string(),
                            }}
                        </span>
                    </div>

                    {move || match active_report.get() {
                        Some(report) => {
                            let is_manual = report.phases.is_empty()
                                && report.anomalies.iter().any(|a| a == "Manual");
                            if is_manual {
                                return view! {
                                    <div class="manual-notice">
                                        "Solve manual sin telemetría BLE (solo tiempo total)"
                                    </div>
                                }
                                .into_any();
                            }

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

                {move || {
                    let is_manual_selected = coach_mode.get() == CoachMode::Solve
                        && active_report
                            .get()
                            .map(|r| r.phases.is_empty() && r.anomalies.iter().any(|a| a == "Manual"))
                            .unwrap_or(false);

                    if is_manual_selected {
                        view! {
                            <div class="radar-card" style="justify-content: center;">
                                <div class="panel-header" style="width: 100%; padding-bottom: 6px;">
                                    <span class="panel-title">"Perfil CFOP"</span>
                                    <span class="solve-meta">"Manual"</span>
                                </div>
                                <div class="manual-notice" style="margin-top: 16px; text-align: center;">
                                    "Solve manual sin telemetría BLE (solo tiempo total)"
                                </div>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! { <CfopRadarChart scores=radar_scores mode_label=radar_mode_label /> }.into_any()
                    }
                }}
            </div>
        </section>
    }
}

#[component]
pub fn RightCoachPanel(
    coach_text: ReadSignal<Option<String>>,
    coach_loading: ReadSignal<bool>,
    coach_mode: ReadSignal<CoachMode>,
    on_change_coach_mode: Callback<CoachMode>,
    has_active_report: Signal<bool>,
    has_session_solves: Signal<bool>,
    on_reanalyze: Callback<()>,
) -> impl IntoView {
    let can_analyze = move || match coach_mode.get() {
        CoachMode::Solve => has_active_report.get(),
        CoachMode::Session => has_session_solves.get(),
    };

    view! {
        <aside class="panel">
            <div class="panel-header" style="flex-wrap: wrap; gap: 6px;">
                <span class="panel-title">"AI Coach Insights"</span>
                <div class="coach-controls">
                    <div class="coach-mode-toggle">
                        <button
                            class=move || if coach_mode.get() == CoachMode::Solve { "mode-btn active" } else { "mode-btn" }
                            on:click=move |_| on_change_coach_mode.run(CoachMode::Solve)
                        >
                            "Solve Actual"
                        </button>
                        <button
                            class=move || if coach_mode.get() == CoachMode::Session { "mode-btn active" } else { "mode-btn" }
                            on:click=move |_| on_change_coach_mode.run(CoachMode::Session)
                        >
                            "Sesión Completa"
                        </button>
                    </div>
                    <button
                        class="btn"
                        style="padding: 4px 10px; font-size: 11px;"
                        disabled=move || coach_loading.get() || !can_analyze()
                        on:click=move |_| on_reanalyze.run(())
                    >
                        {move || if coach_loading.get() { "Analizando..." } else { "Re-analizar" }}
                    </button>
                </div>
            </div>

            <div class="coach-card">
                <div class="coach-content">
                    {move || {
                        if coach_loading.get() {
                            return view! {
                                <div class="coach-placeholder">
                                    "Consultando a Gemini AI Coach..."
                                </div>
                            }
                            .into_any();
                        }

                        match coach_text.get() {
                            Some(text) => {
                                let html = markdown_to_safe_html(&text);
                                view! { <div class="coach-markdown" inner_html=html></div> }.into_any()
                            }
                            None => view! {
                                <div class="coach-placeholder">
                                    "Al terminar un solve o seleccionar 'Sesión Completa', aquí aparecerá el diagnóstico estructurado de Gemini."
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
