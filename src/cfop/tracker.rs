use crate::cfop::metrics::{CfopMetrics, PhaseSummary};
use crate::cfop::phase::CfopPhase;
use crate::cube::color::Face;
use crate::cube::moves::Move;
use crate::cube::state::Cube;

#[derive(Debug, Clone)]
pub struct CfopTracker {
    cube: Cube,
    current_phase: CfopPhase,
    scramble: Vec<Move>,
    inspection_start_ms: Option<u64>,
    phase_start_ms: u64,
    phase_moves: Vec<(Move, u64)>,
    first_effective_move_ms: Option<u64>,
    full_history: Vec<(Move, u64)>,
    metrics: CfopMetrics,
}

impl Default for CfopTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CfopTracker {
    pub fn new() -> Self {
        Self {
            cube: Cube::new(),
            current_phase: CfopPhase::Scramble,
            scramble: Vec::new(),
            inspection_start_ms: None,
            phase_start_ms: 0,
            phase_moves: Vec::new(),
            first_effective_move_ms: None,
            full_history: Vec::new(),
            metrics: CfopMetrics::default(),
        }
    }

    pub fn cube(&self) -> &Cube {
        &self.cube
    }

    pub fn current_phase(&self) -> CfopPhase {
        self.current_phase
    }

    pub fn is_solved(&self) -> bool {
        is_cube_solved(&self.cube)
    }

    pub fn metrics(&self) -> &CfopMetrics {
        &self.metrics
    }

    pub fn full_history(&self) -> &[(Move, u64)] {
        &self.full_history
    }

    /// Aplica un scramble al cubo y prepara la transición a Inspection.
    pub fn apply_scramble(&mut self, scramble_moves: &[Move]) {
        self.cube.apply_moves(scramble_moves);
        self.scramble = scramble_moves.to_vec();
        self.current_phase = CfopPhase::Inspection;
    }

    /// Inicia el tiempo de inspección.
    pub fn start_inspection(&mut self, timestamp_ms: u64) {
        self.inspection_start_ms = Some(timestamp_ms);
        self.current_phase = CfopPhase::Inspection;
    }

    /// Inicia el cronómetro del solve (fin de inspección / inicio de fase Cross).
    pub fn start_solve(&mut self, timestamp_ms: u64) {
        self.current_phase = CfopPhase::Cross;
        self.phase_start_ms = timestamp_ms;
        self.phase_moves.clear();
        self.first_effective_move_ms = None;
    }

    /// Procesa cada giro con su timestamp en milisegundos.
    pub fn on_move(&mut self, m: Move, timestamp_ms: u64) {
        // Si aún estábamos en Scramble o Inspection, el primer giro inicia la fase Cross
        if self.current_phase == CfopPhase::Scramble || self.current_phase == CfopPhase::Inspection {
            self.current_phase = CfopPhase::Cross;
            self.phase_start_ms = timestamp_ms;
            self.phase_moves.clear();
            self.first_effective_move_ms = None;
        }

        if self.current_phase == CfopPhase::Solved {
            return;
        }

        // Aplicar el movimiento al cubo
        self.cube.apply_move(m);
        self.phase_moves.push((m, timestamp_ms));
        self.full_history.push((m, timestamp_ms));

        // Para OLL y PLL, rastreamos el primer giro efectivo (no pre-AUF)
        if (self.current_phase == CfopPhase::OLL || self.current_phase == CfopPhase::PLL)
            && self.first_effective_move_ms.is_none()
            && !m.is_auf()
        {
            self.first_effective_move_ms = Some(timestamp_ms);
        }

        // Evaluar máquina de estados en cascada
        self.evaluate_cascade(timestamp_ms);
    }

    fn evaluate_cascade(&mut self, timestamp_ms: u64) {
        match self.current_phase {
            CfopPhase::Cross => {
                if is_cross_solved(&self.cube) {
                    let summary = self.create_phase_summary(CfopPhase::Cross, timestamp_ms, false);
                    self.metrics.cross = Some(summary);

                    // Contar pares F2L ya completados para detectar X-Cross / XX-Cross
                    let pairs = count_solved_f2l_pairs(&self.cube);
                    self.advance_from_cross(pairs, timestamp_ms);
                }
            }
            CfopPhase::F2L_1 => {
                let pairs = count_solved_f2l_pairs(&self.cube);
                if pairs >= 1 {
                    let summary = self.create_phase_summary(CfopPhase::F2L_1, timestamp_ms, false);
                    self.metrics.f2l_1 = Some(summary);
                    self.phase_start_ms = timestamp_ms;
                    self.phase_moves.clear();
                    self.first_effective_move_ms = None;

                    if pairs == 1 {
                        self.current_phase = CfopPhase::F2L_2;
                    } else if pairs == 2 {
                        self.metrics.f2l_2 = Some(Self::skip_summary(CfopPhase::F2L_2, timestamp_ms));
                        self.current_phase = CfopPhase::F2L_3;
                    } else if pairs == 3 {
                        self.metrics.f2l_2 = Some(Self::skip_summary(CfopPhase::F2L_2, timestamp_ms));
                        self.metrics.f2l_3 = Some(Self::skip_summary(CfopPhase::F2L_3, timestamp_ms));
                        self.current_phase = CfopPhase::F2L_4;
                    } else {
                        self.metrics.f2l_2 = Some(Self::skip_summary(CfopPhase::F2L_2, timestamp_ms));
                        self.metrics.f2l_3 = Some(Self::skip_summary(CfopPhase::F2L_3, timestamp_ms));
                        self.metrics.f2l_4 = Some(Self::skip_summary(CfopPhase::F2L_4, timestamp_ms));
                        self.advance_from_f2l4(timestamp_ms);
                    }
                }
            }
            CfopPhase::F2L_2 => {
                let pairs = count_solved_f2l_pairs(&self.cube);
                if pairs >= 2 {
                    let summary = self.create_phase_summary(CfopPhase::F2L_2, timestamp_ms, false);
                    self.metrics.f2l_2 = Some(summary);
                    self.phase_start_ms = timestamp_ms;
                    self.phase_moves.clear();
                    self.first_effective_move_ms = None;

                    if pairs == 2 {
                        self.current_phase = CfopPhase::F2L_3;
                    } else if pairs == 3 {
                        self.metrics.f2l_3 = Some(Self::skip_summary(CfopPhase::F2L_3, timestamp_ms));
                        self.current_phase = CfopPhase::F2L_4;
                    } else {
                        self.metrics.f2l_3 = Some(Self::skip_summary(CfopPhase::F2L_3, timestamp_ms));
                        self.metrics.f2l_4 = Some(Self::skip_summary(CfopPhase::F2L_4, timestamp_ms));
                        self.advance_from_f2l4(timestamp_ms);
                    }
                }
            }
            CfopPhase::F2L_3 => {
                let pairs = count_solved_f2l_pairs(&self.cube);
                if pairs >= 3 {
                    let summary = self.create_phase_summary(CfopPhase::F2L_3, timestamp_ms, false);
                    self.metrics.f2l_3 = Some(summary);
                    self.phase_start_ms = timestamp_ms;
                    self.phase_moves.clear();
                    self.first_effective_move_ms = None;

                    if pairs == 3 {
                        self.current_phase = CfopPhase::F2L_4;
                    } else {
                        self.metrics.f2l_4 = Some(Self::skip_summary(CfopPhase::F2L_4, timestamp_ms));
                        self.advance_from_f2l4(timestamp_ms);
                    }
                }
            }
            CfopPhase::F2L_4 => {
                if count_solved_f2l_pairs(&self.cube) >= 4 {
                    let summary = self.create_phase_summary(CfopPhase::F2L_4, timestamp_ms, false);
                    self.metrics.f2l_4 = Some(summary);

                    self.advance_from_f2l4(timestamp_ms);
                }
            }
            CfopPhase::OLL => {
                if is_oll_solved(&self.cube) {
                    let summary = self.create_phase_summary(CfopPhase::OLL, timestamp_ms, false);
                    self.metrics.oll = Some(summary);

                    self.advance_from_oll(timestamp_ms);
                }
            }
            CfopPhase::PLL => {
                if is_cube_solved(&self.cube) {
                    let summary = self.create_phase_summary(CfopPhase::PLL, timestamp_ms, false);
                    self.metrics.pll = Some(summary);

                    self.current_phase = CfopPhase::Solved;
                    self.metrics.total_time_ms = timestamp_ms.saturating_sub(
                        self.metrics
                            .cross
                            .as_ref()
                            .map(|c| c.start_time_ms)
                            .unwrap_or(self.phase_start_ms),
                    );
                }
            }
            CfopPhase::Scramble | CfopPhase::Inspection | CfopPhase::Solved => {}
        }
    }

    fn advance_from_cross(&mut self, pairs: usize, timestamp_ms: u64) {
        if pairs >= 1 {
            self.metrics.is_x_cross = true;
            self.metrics.f2l_1 = Some(Self::skip_summary(CfopPhase::F2L_1, timestamp_ms));
        }
        if pairs >= 2 {
            self.metrics.is_xx_cross = true;
            self.metrics.f2l_2 = Some(Self::skip_summary(CfopPhase::F2L_2, timestamp_ms));
        }
        if pairs >= 3 {
            self.metrics.f2l_3 = Some(Self::skip_summary(CfopPhase::F2L_3, timestamp_ms));
        }
        if pairs >= 4 {
            self.metrics.f2l_4 = Some(Self::skip_summary(CfopPhase::F2L_4, timestamp_ms));
            self.advance_from_f2l4(timestamp_ms);
            return;
        }

        self.phase_start_ms = timestamp_ms;
        self.phase_moves.clear();
        self.first_effective_move_ms = None;

        match pairs {
            0 => self.current_phase = CfopPhase::F2L_1,
            1 => self.current_phase = CfopPhase::F2L_2,
            2 => self.current_phase = CfopPhase::F2L_3,
            3 => self.current_phase = CfopPhase::F2L_4,
            _ => {}
        }
    }



    fn advance_from_f2l4(&mut self, timestamp_ms: u64) {
        // Verificar OLL Skip (cara superior ya orientada tras el 4to par)
        if is_oll_solved(&self.cube) {
            self.metrics.is_oll_skip = true;
            self.metrics.oll = Some(Self::skip_summary(CfopPhase::OLL, timestamp_ms));

            self.advance_from_oll(timestamp_ms);
        } else {
            self.current_phase = CfopPhase::OLL;
            self.phase_start_ms = timestamp_ms;
            self.phase_moves.clear();
            self.first_effective_move_ms = None;
        }
    }

    fn advance_from_oll(&mut self, timestamp_ms: u64) {
        // Verificar PLL Skip (queda resuelto tras OLL)
        if is_cube_solved(&self.cube) {
            self.metrics.is_pll_skip = true;
            self.metrics.pll = Some(Self::skip_summary(CfopPhase::PLL, timestamp_ms));

            self.current_phase = CfopPhase::Solved;
            self.metrics.total_time_ms = timestamp_ms.saturating_sub(
                self.metrics
                    .cross
                    .as_ref()
                    .map(|c| c.start_time_ms)
                    .unwrap_or(self.phase_start_ms),
            );
        } else {
            self.current_phase = CfopPhase::PLL;
            self.phase_start_ms = timestamp_ms;
            self.phase_moves.clear();
            self.first_effective_move_ms = None;
        }
    }

    fn skip_summary(phase: CfopPhase, timestamp_ms: u64) -> PhaseSummary {
        let (rec, exec) = if phase == CfopPhase::OLL || phase == CfopPhase::PLL {
            (Some(0), Some(0))
        } else {
            (None, None)
        };

        PhaseSummary {
            phase,
            start_time_ms: timestamp_ms,
            end_time_ms: timestamp_ms,
            duration_ms: 0,
            move_count: 0,
            moves: Vec::new(),
            is_skip: true,
            recognition_time_ms: rec,
            execution_time_ms: exec,
            pre_auf: Vec::new(),
            post_auf: Vec::new(),
        }
    }

    fn create_phase_summary(
        &self,
        phase: CfopPhase,
        end_time_ms: u64,
        is_skip: bool,
    ) -> PhaseSummary {
        let duration_ms = end_time_ms.saturating_sub(self.phase_start_ms);
        let moves_only: Vec<Move> = self.phase_moves.iter().map(|(m, _)| *m).collect();
        let move_count = moves_only.len();

        let mut rec_time = None;
        let mut exec_time = None;
        let mut pre_auf = Vec::new();
        let mut post_auf = Vec::new();

        if phase == CfopPhase::OLL || phase == CfopPhase::PLL {
            // Aislar giros de ajuste AUF (U, U', U2) iniciales y finales
            let mut first_non_auf_idx = None;
            let mut last_non_auf_idx = None;

            for (i, &m) in moves_only.iter().enumerate() {
                if !m.is_auf() {
                    if first_non_auf_idx.is_none() {
                        first_non_auf_idx = Some(i);
                    }
                    last_non_auf_idx = Some(i);
                }
            }

            if let Some(first_idx) = first_non_auf_idx {
                pre_auf = moves_only[..first_idx].to_vec();
                let last_idx = last_non_auf_idx.unwrap();
                post_auf = moves_only[last_idx + 1..].to_vec();

                let first_effective_time = self.phase_moves[first_idx].1;
                let rec = first_effective_time.saturating_sub(self.phase_start_ms);
                let exec = end_time_ms.saturating_sub(first_effective_time);

                rec_time = Some(rec);
                exec_time = Some(exec);
            } else {
                // Solo hubo giros AUF
                pre_auf = moves_only.clone();
                if let Some(&(first_m, first_t)) = self.phase_moves.first() {
                    let _ = first_m;
                    let rec = first_t.saturating_sub(self.phase_start_ms);
                    let exec = end_time_ms.saturating_sub(first_t);
                    rec_time = Some(rec);
                    exec_time = Some(exec);
                } else {
                    rec_time = Some(0);
                    exec_time = Some(0);
                }
            }
        }

        PhaseSummary {
            phase,
            start_time_ms: self.phase_start_ms,
            end_time_ms,
            duration_ms,
            move_count,
            moves: moves_only,
            is_skip,
            recognition_time_ms: rec_time,
            execution_time_ms: exec_time,
            pre_auf,
            post_auf,
        }
    }
}

pub fn is_cross_solved(cube: &Cube) -> bool {
    let stickers = cube.to_stickers();
    let d_center = cube.centers[Face::D as usize];

    // DF: D index 1, F index 7
    let df_ok = stickers[Face::D as usize * 9 + 1] == d_center
        && stickers[Face::F as usize * 9 + 7] == cube.centers[Face::F as usize];
    // DR: D index 5, R index 7
    let dr_ok = stickers[Face::D as usize * 9 + 5] == d_center
        && stickers[Face::R as usize * 9 + 7] == cube.centers[Face::R as usize];
    // DB: D index 7, B index 7
    let db_ok = stickers[Face::D as usize * 9 + 7] == d_center
        && stickers[Face::B as usize * 9 + 7] == cube.centers[Face::B as usize];
    // DL: D index 3, L index 7
    let dl_ok = stickers[Face::D as usize * 9 + 3] == d_center
        && stickers[Face::L as usize * 9 + 7] == cube.centers[Face::L as usize];

    df_ok && dr_ok && db_ok && dl_ok
}

pub fn count_solved_f2l_pairs(cube: &Cube) -> usize {
    let stickers = cube.to_stickers();
    let d_center = cube.centers[Face::D as usize];
    let f_center = cube.centers[Face::F as usize];
    let b_center = cube.centers[Face::B as usize];
    let l_center = cube.centers[Face::L as usize];
    let r_center = cube.centers[Face::R as usize];

    let mut count = 0;

    // Pair 0: FL (DFL corner + FL edge)
    // Corner DFL: D0, L8, F6
    let dfl_corner_ok = stickers[Face::D as usize * 9 + 0] == d_center
        && stickers[Face::L as usize * 9 + 8] == l_center
        && stickers[Face::F as usize * 9 + 6] == f_center;
    // Edge FL: F3, L5
    let fl_edge_ok = stickers[Face::F as usize * 9 + 3] == f_center
        && stickers[Face::L as usize * 9 + 5] == l_center;
    if dfl_corner_ok && fl_edge_ok {
        count += 1;
    }

    // Pair 1: FR (DFR corner + FR edge)
    // Corner DFR: D2, F8, R6
    let dfr_corner_ok = stickers[Face::D as usize * 9 + 2] == d_center
        && stickers[Face::F as usize * 9 + 8] == f_center
        && stickers[Face::R as usize * 9 + 6] == r_center;
    // Edge FR: F5, R3
    let fr_edge_ok = stickers[Face::F as usize * 9 + 5] == f_center
        && stickers[Face::R as usize * 9 + 3] == r_center;
    if dfr_corner_ok && fr_edge_ok {
        count += 1;
    }

    // Pair 2: BR (DBR corner + BR edge)
    // Corner DBR: D8, R8, B6
    let dbr_corner_ok = stickers[Face::D as usize * 9 + 8] == d_center
        && stickers[Face::R as usize * 9 + 8] == r_center
        && stickers[Face::B as usize * 9 + 6] == b_center;
    // Edge BR: B3, R5
    let br_edge_ok = stickers[Face::B as usize * 9 + 3] == b_center
        && stickers[Face::R as usize * 9 + 5] == r_center;
    if dbr_corner_ok && br_edge_ok {
        count += 1;
    }

    // Pair 3: BL (DBL corner + BL edge)
    // Corner DBL: D6, B8, L6
    let dbl_corner_ok = stickers[Face::D as usize * 9 + 6] == d_center
        && stickers[Face::B as usize * 9 + 8] == b_center
        && stickers[Face::L as usize * 9 + 6] == l_center;
    // Edge BL: B5, L3
    let bl_edge_ok = stickers[Face::B as usize * 9 + 5] == b_center
        && stickers[Face::L as usize * 9 + 3] == l_center;
    if dbl_corner_ok && bl_edge_ok {
        count += 1;
    }

    count
}

pub fn is_oll_solved(cube: &Cube) -> bool {
    let stickers = cube.to_stickers();
    let u_center = cube.centers[Face::U as usize];
    let u_base = Face::U as usize * 9;
    (0..9).all(|i| stickers[u_base + i] == u_center)
}

pub fn is_cube_solved(cube: &Cube) -> bool {
    let stickers = cube.to_stickers();
    for face in Face::ALL {
        let base = face as usize * 9;
        let center_color = cube.centers[face as usize];
        if !(0..9).all(|i| stickers[base + i] == center_color) {
            return false;
        }
    }
    true
}
