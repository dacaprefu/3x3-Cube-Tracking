use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTab {
    Dashboard,
    History,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubeMovePayload {
    pub move_notation: String,
    pub stickers: Vec<String>,
    pub phase: String,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubeOrientationDto {
    pub up: String,
    pub front: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CubeTelemetryPayload {
    pub battery: Option<u8>,
    pub orientation: CubeOrientationDto,
    pub quaternion: [f32; 4],
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseMetricsDto {
    pub phase: String,
    pub move_count: usize,
    pub duration_ms: u64,
    pub active_time_ms: u64,
    pub tps_raw: f32,
    pub tps_effective: f32,
    pub y_rotations: usize,
    pub is_inefficient: bool,
    pub recognition_time_ms: Option<u64>,
    pub execution_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotationTaxDto {
    pub total_y_rotations_f2l: usize,
    pub rotation_dead_time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveReportDto {
    pub total_time_ms: u64,
    pub active_time_ms: u64,
    pub total_moves_htm: usize,
    pub tps_raw: f32,
    pub tps_effective: f32,
    pub phases: Vec<PhaseMetricsDto>,
    pub rotation_tax: RotationTaxDto,
    pub anomalies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolveEntry {
    pub id: usize,
    pub scramble: String,
    pub report: SolveReportDto,
    pub raw_json: serde_json::Value,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct BleUiState {
    pub connected: bool,
    pub battery: Option<u8>,
    pub latency_ms: Option<u64>,
}

pub fn default_solved_stickers() -> Vec<String> {
    let face_colors = ["White", "Yellow", "Green", "Blue", "Orange", "Red"];
    let mut out = Vec::with_capacity(54);
    for color in face_colors {
        for _ in 0..9 {
            out.push(color.to_string());
        }
    }
    out
}

pub fn sticker_hex(color: &str) -> &'static str {
    match color {
        "White" => "#f8fafc",
        "Yellow" => "#facc15",
        "Green" => "#22c55e",
        "Blue" => "#2563eb",
        "Orange" => "#f97316",
        "Red" => "#dc2626",
        _ => "#334155",
    }
}

pub fn phase_label(phase: &str) -> &str {
    match phase {
        "Cross" => "Cruz",
        "F2L_1" => "F2L 1",
        "F2L_2" => "F2L 2",
        "F2L_3" => "F2L 3",
        "F2L_4" => "F2L 4",
        "OLL" => "OLL",
        "PLL" => "PLL",
        "Solved" => "Resuelto",
        "Inspection" => "Inspección",
        "Scramble" => "Scramble",
        other => other,
    }
}

pub fn format_ms_stopwatch(ms: u64) -> String {
    let minutes = ms / 60_000;
    let seconds = (ms % 60_000) / 1_000;
    let millis = ms % 1_000;
    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

pub fn calc_trimmed_ao(solves: &[SolveEntry], n: usize) -> Option<u64> {
    if solves.len() < n || n < 3 {
        return None;
    }
    let start = solves.len() - n;
    let mut times: Vec<u64> = solves[start..]
        .iter()
        .map(|s| s.report.total_time_ms)
        .collect();
    times.sort_unstable();
    let trimmed = &times[1..times.len() - 1];
    let sum: u64 = trimmed.iter().sum();
    Some(sum / (trimmed.len() as u64))
}

pub fn calc_pb(solves: &[SolveEntry]) -> Option<u64> {
    solves.iter().map(|s| s.report.total_time_ms).min()
}
