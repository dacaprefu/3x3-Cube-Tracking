use crate::cfop::phase::CfopPhase;
use crate::cube::moves::Move;
use crate::cube::state::Cube;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseRecord {
    pub duration_ms: u64,
    pub phase: CfopPhase,
    pub start_timestamp_ms: u64,
    pub end_timestamp_ms: u64,
    pub cube_state: Cube,
    pub previous_move: Option<Move>,
    pub next_move: Move,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlindRotation {
    pub first_rot: Move,
    pub second_rot: Move,
    pub phase: CfopPhase,
    pub time_between_ms: u64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotationTax {
    pub total_y_rotations_f2l: usize,
    pub rotation_dead_time_ms: u64,
    pub blind_rotations: Vec<BlindRotation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockupRecord {
    pub phase: CfopPhase,
    pub duration_ms: u64,
    pub trigger_name: String,
    pub move_index: usize,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseMetrics {
    pub phase: CfopPhase,
    pub move_count: usize,
    pub duration_ms: u64,
    pub active_time_ms: u64,
    pub tps_raw: f32,
    pub tps_effective: f32,
    pub pauses: Vec<PauseRecord>,
    pub y_rotations: usize,
    pub is_inefficient: bool,
    pub recognition_time_ms: Option<u64>,
    pub execution_time_ms: Option<u64>,
    pub pre_auf: Vec<Move>,
    pub post_auf: Vec<Move>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveReport {
    pub total_time_ms: u64,
    pub active_time_ms: u64,
    pub total_moves_htm: usize,
    pub tps_raw: f32,
    pub tps_effective: f32,
    pub phases: Vec<PhaseMetrics>,
    pub pauses: Vec<PauseRecord>,
    pub rotation_tax: RotationTax,
    pub lockups: Vec<LockupRecord>,
    pub anomalies: Vec<String>,
}
