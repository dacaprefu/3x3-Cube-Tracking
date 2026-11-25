use crate::cfop::phase::CfopPhase;
use crate::cube::moves::Move;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseSummary {
    pub phase: CfopPhase,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub duration_ms: u64,
    pub move_count: usize,
    pub moves: Vec<Move>,
    pub is_skip: bool,

    // Separación para OLL y PLL
    pub recognition_time_ms: Option<u64>,
    pub execution_time_ms: Option<u64>,
    pub pre_auf: Vec<Move>,
    pub post_auf: Vec<Move>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CfopMetrics {
    pub cross: Option<PhaseSummary>,
    pub f2l_1: Option<PhaseSummary>,
    pub f2l_2: Option<PhaseSummary>,
    pub f2l_3: Option<PhaseSummary>,
    pub f2l_4: Option<PhaseSummary>,
    pub oll: Option<PhaseSummary>,
    pub pll: Option<PhaseSummary>,

    pub is_x_cross: bool,
    pub is_xx_cross: bool,
    pub is_oll_skip: bool,
    pub is_pll_skip: bool,

    pub total_time_ms: u64,
}
