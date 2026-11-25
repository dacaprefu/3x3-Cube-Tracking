use crate::analytics::error::AnalyticsError;
use crate::analytics::models::{
    BlindRotation, LockupRecord, PauseRecord, PhaseMetrics, RotationTax, SolveReport,
};
use crate::cfop::metrics::PhaseSummary;
use crate::cfop::phase::CfopPhase;
use crate::cfop::tracker::CfopTracker;
use crate::cube::moves::Move;
use crate::cube::state::Cube;

pub fn phase_str(phase: CfopPhase) -> &'static str {
    match phase {
        CfopPhase::Cross => "Cross",
        CfopPhase::F2L_1 => "F2L-1",
        CfopPhase::F2L_2 => "F2L-2",
        CfopPhase::F2L_3 => "F2L-3",
        CfopPhase::F2L_4 => "F2L-4",
        CfopPhase::OLL => "OLL",
        CfopPhase::PLL => "PLL",
        CfopPhase::Inspection => "Inspection",
        CfopPhase::Scramble => "Scramble",
        CfopPhase::Solved => "Solved",
    }
}

fn is_f2l_phase(phase: CfopPhase) -> bool {
    matches!(
        phase,
        CfopPhase::F2L_1 | CfopPhase::F2L_2 | CfopPhase::F2L_3 | CfopPhase::F2L_4
    )
}

fn is_y_rotation(m: Move) -> bool {
    matches!(m, Move::Y | Move::YPrime | Move::Y2)
}

#[derive(Debug, Clone)]
pub struct SolveAnalyzer {
    pub critical_pause_threshold_ms: u64,
    pub lockup_threshold_ms: u64,
    pub f2l_max_efficient_moves: usize,
    pub recognition_doubt_threshold_ms: u64,
}

impl Default for SolveAnalyzer {
    fn default() -> Self {
        Self {
            critical_pause_threshold_ms: 350,
            lockup_threshold_ms: 180,
            f2l_max_efficient_moves: 11,
            recognition_doubt_threshold_ms: 700,
        }
    }
}

impl SolveAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analiza el solve completo a partir del tracker y genera el `SolveReport` con sus métricas y anomalías.
    pub fn analyze(&self, tracker: &CfopTracker) -> Result<SolveReport, AnalyticsError> {
        let history = tracker.full_history();
        if history.is_empty() {
            return Err(AnalyticsError::InsufficientMoves);
        }

        let metrics = tracker.metrics();
        let total_time_ms = metrics.total_time_ms;
        if total_time_ms == 0 {
            return Err(AnalyticsError::SolveIncomplete);
        }

        let mut pauses = Vec::new();
        let mut lockups = Vec::new();
        let mut blind_rotations = Vec::new();
        let mut total_y_rotations_f2l = 0;
        let mut rotation_dead_time_ms = 0;
        let mut anomalies = Vec::new();

        // Reconstrucción del estado del cubo paso a paso para guardar el estado exacto en pausas
        let mut replay_cube = Cube::new();

        // Recorrer el historial de movimientos
        for i in 0..history.len() {
            let (m, t) = history[i];
            let delta_t = if i == 0 {
                0
            } else {
                t.saturating_sub(history[i - 1].1)
            };

            let phase_at_move = get_phase_at_timestamp(metrics, t);

            // 1. Detector de Pausas (Lookahead)
            if i > 0 && delta_t > self.critical_pause_threshold_ms {
                let pause_start = history[i - 1].1;
                let phase_at_pause = get_phase_at_timestamp(metrics, pause_start);

                pauses.push(PauseRecord {
                    duration_ms: delta_t,
                    phase: phase_at_pause,
                    start_timestamp_ms: pause_start,
                    end_timestamp_ms: t,
                    cube_state: replay_cube.clone(),
                    previous_move: Some(history[i - 1].0),
                    next_move: m,
                });

                anomalies.push(format!(
                    "[PAUSA_CRITICA: {}ms en {}]",
                    delta_t,
                    phase_str(phase_at_pause)
                ));
            }

            // 2. Impuesto de Rotaciones y Blind Rotations
            if is_f2l_phase(phase_at_move) && is_y_rotation(m) {
                total_y_rotations_f2l += 1;
                rotation_dead_time_ms += delta_t;

                // Detectar Blind Rotation: dos rotaciones consecutivas sin giros de capa entre ellas
                if i > 0 && is_y_rotation(history[i - 1].0) {
                    blind_rotations.push(BlindRotation {
                        first_rot: history[i - 1].0,
                        second_rot: m,
                        phase: phase_at_move,
                        time_between_ms: delta_t,
                        timestamp_ms: t,
                    });

                    anomalies.push(format!(
                        "[BLIND_ROTATION: {} + {} en {}]",
                        history[i - 1].0,
                        m,
                        phase_str(phase_at_move)
                    ));
                }
            }

            // 3. Detector de Lockups (Atascos)
            // Identifica picos anómalos de delta_t (> 180ms) dentro de secuencias de giro rápido
            if i > 0 && delta_t > self.lockup_threshold_ms && delta_t <= self.critical_pause_threshold_ms {
                // Comprobar si los giros adyacentes son rápidos (secuencia rápida / trigger)
                let prev_fast = if i >= 2 {
                    history[i - 1].1.saturating_sub(history[i - 2].1) <= 160
                } else {
                    false
                };
                let next_fast = if i + 1 < history.len() {
                    history[i + 1].1.saturating_sub(t) <= 160
                } else {
                    false
                };

                // Si ocurrió dentro de una secuencia rápida o en fase OLL/PLL/F2L
                if prev_fast || next_fast || matches!(phase_at_move, CfopPhase::OLL | CfopPhase::PLL) {
                    lockups.push(LockupRecord {
                        phase: phase_at_move,
                        duration_ms: delta_t,
                        trigger_name: format!("Move {}", m),
                        move_index: i,
                        timestamp_ms: t,
                    });

                    anomalies.push(format!(
                        "[LOCKUP_{}: {}ms]",
                        phase_str(phase_at_move),
                        delta_t
                    ));
                }
            }

            // Aplicar el movimiento al cubo de replay
            replay_cube.apply_move(m);
        }

        // 4. Eficiencia HTM y TPS
        let total_moves_htm = history.iter().filter(|(m, _)| !m.is_rotation()).count();
        let tps_raw = (total_moves_htm as f32) / (total_time_ms as f32 / 1000.0);

        let total_pause_time_ms: u64 = pauses.iter().map(|p| p.duration_ms).sum();
        let active_time_ms = total_time_ms.saturating_sub(total_pause_time_ms);
        let tps_effective = if active_time_ms > 0 {
            (total_moves_htm as f32) / (active_time_ms as f32 / 1000.0)
        } else {
            tps_raw
        };

        // 5. Análisis por fase e ineficiencia F2L
        let mut phase_metrics = Vec::new();
        let phase_summaries = [
            metrics.cross.as_ref(),
            metrics.f2l_1.as_ref(),
            metrics.f2l_2.as_ref(),
            metrics.f2l_3.as_ref(),
            metrics.f2l_4.as_ref(),
            metrics.oll.as_ref(),
            metrics.pll.as_ref(),
        ];

        for summary_opt in phase_summaries.into_iter().flatten() {
            let phase_pauses: Vec<PauseRecord> = pauses
                .iter()
                .filter(|p| p.phase == summary_opt.phase)
                .cloned()
                .collect();

            let phase_pause_time: u64 = phase_pauses.iter().map(|p| p.duration_ms).sum();
            let phase_active_time = summary_opt.duration_ms.saturating_sub(phase_pause_time);

            let phase_tps_raw = if summary_opt.duration_ms > 0 {
                (summary_opt.move_count as f32) / (summary_opt.duration_ms as f32 / 1000.0)
            } else {
                0.0
            };

            let phase_tps_eff = if phase_active_time > 0 {
                (summary_opt.move_count as f32) / (phase_active_time as f32 / 1000.0)
            } else {
                phase_tps_raw
            };

            let is_inefficient = is_f2l_phase(summary_opt.phase)
                && summary_opt.move_count > self.f2l_max_efficient_moves;

            if is_inefficient {
                anomalies.push(format!(
                    "[INEFICIENCIA_F2L: {} giros en {}]",
                    summary_opt.move_count,
                    phase_str(summary_opt.phase)
                ));
            }

            // Duda de reconocimiento en PLL y OLL
            if summary_opt.phase == CfopPhase::PLL || summary_opt.phase == CfopPhase::OLL {
                self.check_recognition_doubt(summary_opt, &mut anomalies);
            }

            phase_metrics.push(PhaseMetrics {
                phase: summary_opt.phase,
                move_count: summary_opt.move_count,
                duration_ms: summary_opt.duration_ms,
                active_time_ms: phase_active_time,
                tps_raw: phase_tps_raw,
                tps_effective: phase_tps_eff,
                pauses: phase_pauses,
                y_rotations: summary_opt.moves.iter().filter(|&&m| is_y_rotation(m)).count(),
                is_inefficient,
                recognition_time_ms: summary_opt.recognition_time_ms,
                execution_time_ms: summary_opt.execution_time_ms,
                pre_auf: summary_opt.pre_auf.clone(),
                post_auf: summary_opt.post_auf.clone(),
            });
        }

        let rotation_tax = RotationTax {
            total_y_rotations_f2l,
            rotation_dead_time_ms,
            blind_rotations,
        };

        Ok(SolveReport {
            total_time_ms,
            active_time_ms,
            total_moves_htm,
            tps_raw,
            tps_effective,
            phases: phase_metrics,
            pauses,
            rotation_tax,
            lockups,
            anomalies,
        })
    }

    fn check_recognition_doubt(&self, summary: &PhaseSummary, anomalies: &mut Vec<String>) {
        let rec = summary.recognition_time_ms.unwrap_or(0);
        let has_double_auf = summary.pre_auf.len() >= 2;
        let high_rec = rec >= self.recognition_doubt_threshold_ms;

        let phase_name = phase_str(summary.phase);

        if high_rec && has_double_auf {
            anomalies.push(format!(
                "[DUDA_RECONOCIMIENTO_{}: {}ms + doble AUF]",
                phase_name, rec
            ));
        } else if high_rec {
            anomalies.push(format!(
                "[DUDA_RECONOCIMIENTO_{}: {}ms]",
                phase_name, rec
            ));
        } else if has_double_auf {
            anomalies.push(format!(
                "[DUDA_RECONOCIMIENTO_{}: doble AUF]",
                phase_name
            ));
        }
    }
}

fn get_phase_at_timestamp(
    metrics: &crate::cfop::metrics::CfopMetrics,
    t: u64,
) -> CfopPhase {
    if let Some(cross) = &metrics.cross {
        if t <= cross.end_time_ms {
            return CfopPhase::Cross;
        }
    }
    if let Some(f1) = &metrics.f2l_1 {
        if t <= f1.end_time_ms {
            return CfopPhase::F2L_1;
        }
    }
    if let Some(f2) = &metrics.f2l_2 {
        if t <= f2.end_time_ms {
            return CfopPhase::F2L_2;
        }
    }
    if let Some(f3) = &metrics.f2l_3 {
        if t <= f3.end_time_ms {
            return CfopPhase::F2L_3;
        }
    }
    if let Some(f4) = &metrics.f2l_4 {
        if t <= f4.end_time_ms {
            return CfopPhase::F2L_4;
        }
    }
    if let Some(oll) = &metrics.oll {
        if t <= oll.end_time_ms {
            return CfopPhase::OLL;
        }
    }
    if let Some(pll) = &metrics.pll {
        if t <= pll.end_time_ms {
            return CfopPhase::PLL;
        }
    }
    CfopPhase::Solved
}
