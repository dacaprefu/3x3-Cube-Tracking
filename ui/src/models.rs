use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTab {
    Dashboard,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoachMode {
    Solve,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Holding,
    Ready,
    Running,
    Stopped,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrambleStateDto {
    pub moves: Vec<String>,
    pub completed_count: usize,
    pub correction_move: Option<String>,
    pub is_scrambled_ready: bool,
    #[serde(default)]
    pub has_prev: bool,
    #[serde(default)]
    pub is_reviewing_previous: bool,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseSummaryDto {
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveReportDto {
    pub total_time_ms: u64,
    pub active_time_ms: u64,
    pub total_moves_htm: usize,
    pub tps_raw: f32,
    pub tps_effective: f32,
    pub phases: Vec<PhaseMetricsDto>,
    #[serde(default)]
    pub pauses: Vec<PauseSummaryDto>,
    pub rotation_tax: RotationTaxDto,
    #[serde(default)]
    pub lockups: Vec<serde_json::Value>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CfopRadarScores {
    pub cross: f64,
    pub f2l: f64,
    pub oll: f64,
    pub pll: f64,
    pub lookahead: f64,
    pub fluency: f64,
}

pub fn compute_radar_scores(reports: &[SolveReportDto]) -> CfopRadarScores {
    let ble_reports: Vec<&SolveReportDto> = reports
        .iter()
        .filter(|r| !r.phases.is_empty())
        .collect();
    if ble_reports.is_empty() {
        return CfopRadarScores::default();
    }

    let n = ble_reports.len() as f64;
    let mut cross_sum = 0.0;
    let mut f2l_sum = 0.0;
    let mut oll_sum = 0.0;
    let mut pll_sum = 0.0;
    let mut lookahead_sum = 0.0;
    let mut fluency_sum = 0.0;

    for r in ble_reports {
        let cross_score = r
            .phases
            .iter()
            .find(|p| p.phase == "Cross")
            .map(|p| {
                let time_pen = ((p.duration_ms as f64 - 800.0).max(0.0)) / 35.0;
                let move_pen = ((p.move_count as f64 - 8.0).max(0.0)) * 6.0;
                (100.0 - time_pen - move_pen).clamp(10.0, 100.0)
            })
            .unwrap_or(75.0);

        let f2l_phases: Vec<&PhaseMetricsDto> = r
            .phases
            .iter()
            .filter(|p| matches!(p.phase.as_str(), "F2L_1" | "F2L_2" | "F2L_3" | "F2L_4"))
            .collect();
        let f2l_score = if f2l_phases.is_empty() {
            75.0
        } else {
            let avg_tps: f64 = f2l_phases
                .iter()
                .map(|p| p.tps_effective as f64)
                .sum::<f64>()
                / (f2l_phases.len() as f64);
            (avg_tps / 6.0 * 100.0).clamp(15.0, 100.0)
        };

        let oll_score = r
            .phases
            .iter()
            .find(|p| p.phase == "OLL")
            .map(|p| {
                let pen = ((p.duration_ms as f64 - 700.0).max(0.0)) / 25.0;
                (100.0 - pen).clamp(15.0, 100.0)
            })
            .unwrap_or(85.0);

        let pll_score = r
            .phases
            .iter()
            .find(|p| p.phase == "PLL")
            .map(|p| {
                let pen = ((p.duration_ms as f64 - 900.0).max(0.0)) / 30.0;
                (100.0 - pen).clamp(15.0, 100.0)
            })
            .unwrap_or(85.0);

        let crit_pauses = r.pauses.iter().filter(|p| p.duration_ms > 350).count() as f64;
        let lookahead_score = (100.0 - crit_pauses * 18.0).clamp(10.0, 100.0);

        let rots = r.rotation_tax.total_y_rotations_f2l as f64;
        let lockups = r.lockups.len() as f64;
        let fluency_score = (100.0 - rots * 8.0 - lockups * 12.0).clamp(10.0, 100.0);

        cross_sum += cross_score;
        f2l_sum += f2l_score;
        oll_sum += oll_score;
        pll_sum += pll_score;
        lookahead_sum += lookahead_score;
        fluency_sum += fluency_score;
    }

    CfopRadarScores {
        cross: cross_sum / n,
        f2l: f2l_sum / n,
        oll: oll_sum / n,
        pll: pll_sum / n,
        lookahead: lookahead_sum / n,
        fluency: fluency_sum / n,
    }
}

pub fn markdown_to_safe_html(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, options).filter(|event| {
        !matches!(
            event,
            Event::Html(_) | Event::InlineHtml(_) | Event::Start(Tag::HtmlBlock)
        )
    });
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    html_output
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

pub const AVAILABLE_AO_SIZES: [usize; 7] = [5, 12, 50, 100, 250, 500, 1000];

pub fn wca_trim_count(n: usize) -> usize {
    ((n as f64) * 0.05).ceil() as usize
}

pub fn calc_ao_window(times: &[u64]) -> Option<u64> {
    let n = times.len();
    if n < 3 {
        return None;
    }
    let trim = wca_trim_count(n);
    if 2 * trim >= n {
        return None;
    }
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    let kept = &sorted[trim..(n - trim)];
    let sum: u64 = kept.iter().sum();
    Some(sum / (kept.len() as u64))
}

pub fn calc_ao_n(solves: &[SolveEntry], n: usize) -> Option<u64> {
    if solves.len() < n {
        return None;
    }
    let start = solves.len() - n;
    let times: Vec<u64> = solves[start..]
        .iter()
        .map(|s| s.report.total_time_ms)
        .collect();
    calc_ao_window(&times)
}

pub fn calc_best_ao_n(solves: &[SolveEntry], n: usize) -> Option<u64> {
    if solves.len() < n || n < 3 {
        return None;
    }
    let all_times: Vec<u64> = solves.iter().map(|s| s.report.total_time_ms).collect();
    all_times
        .windows(n)
        .filter_map(calc_ao_window)
        .min()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    #[default]
    DarkBlue,
    Gray,
    Light,
}

impl AppTheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DarkBlue => "dark-blue",
            Self::Gray => "gray",
            Self::Light => "light",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "gray" => Self::Gray,
            "light" => Self::Light,
            _ => Self::DarkBlue,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::DarkBlue => "Azul",
            Self::Gray => "Gris",
            Self::Light => "Blanco",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistributionStepMode {
    #[default]
    Auto,
    HalfSec,
    OneSec,
    TwoSec,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeBucket {
    pub label: String,
    pub cum_label: String,
    pub cum_count: usize,
    pub total_count: usize,
    pub range_tooltip: String,
    pub count: usize,
    pub percent: f64,
    pub has_pb: bool,
    pub is_mode: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimeDistribution {
    pub buckets: Vec<TimeBucket>,
    pub max_count: usize,
    pub mean_ms: Option<u64>,
    pub std_dev_ms: Option<u64>,
    pub step_ms: u64,
}

pub fn compute_time_distribution(
    solves: &[SolveEntry],
    mode: DistributionStepMode,
) -> TimeDistribution {
    if solves.is_empty() {
        return TimeDistribution::default();
    }

    let times_ms: Vec<u64> = solves.iter().map(|s| s.report.total_time_ms).collect();
    let total = times_ms.len();
    let sum_f: f64 = times_ms.iter().map(|&t| t as f64).sum();
    let mean_f = sum_f / (total as f64);
    let variance_f = times_ms
        .iter()
        .map(|&t| {
            let diff = t as f64 - mean_f;
            diff * diff
        })
        .sum::<f64>()
        / (total as f64);

    let min_ms = times_ms.iter().copied().min().unwrap_or(0);
    let max_ms = times_ms.iter().copied().max().unwrap_or(0);
    let span_ms = max_ms.saturating_sub(min_ms);

    let step_ms: u64 = match mode {
        DistributionStepMode::HalfSec => 500,
        DistributionStepMode::OneSec => 1000,
        DistributionStepMode::TwoSec => 2000,
        DistributionStepMode::Auto => {
            if span_ms <= 3500 {
                500
            } else if span_ms <= 7000 {
                1000
            } else if span_ms <= 16000 {
                2000
            } else {
                5000
            }
        }
    };

    let start_ms = (min_ms / step_ms) * step_ms;
    let end_ms = (max_ms / step_ms) * step_ms;

    let mut buckets = Vec::new();
    let mut cum_count: usize = 0;
    let mut b_start_ms = start_ms;

    while b_start_ms <= end_ms {
        let b_end_ms = b_start_ms + step_ms;
        let count = times_ms
            .iter()
            .filter(|&&t| t >= b_start_ms && t < b_end_ms)
            .count();
        cum_count += count;

        let (label, cum_label) = if step_ms == 500 {
            (
                format!("{:.1}+", b_start_ms as f64 / 1000.0),
                format!("< {:.1}", b_end_ms as f64 / 1000.0),
            )
        } else {
            (
                format!("{}+", b_start_ms / 1000),
                format!("< {}", b_end_ms / 1000),
            )
        };

        let range_tooltip = format!(
            "{:.2}s – {:.2}s (< {:.2}s)",
            b_start_ms as f64 / 1000.0,
            (b_end_ms.saturating_sub(10)) as f64 / 1000.0,
            b_end_ms as f64 / 1000.0
        );
        let percent = (count as f64 / total as f64) * 100.0;
        let has_pb = min_ms >= b_start_ms && min_ms < b_end_ms;

        buckets.push(TimeBucket {
            label,
            cum_label,
            cum_count,
            total_count: total,
            range_tooltip,
            count,
            percent,
            has_pb,
            is_mode: false,
        });

        b_start_ms += step_ms;
    }

    buckets.retain(|b| b.count > 0);

    let max_count = buckets.iter().map(|b| b.count).max().unwrap_or(0);
    for b in &mut buckets {
        if max_count > 0 && b.count == max_count {
            b.is_mode = true;
        }
    }

    TimeDistribution {
        buckets,
        max_count,
        mean_ms: Some(mean_f.round() as u64),
        std_dev_ms: Some(variance_f.sqrt().round() as u64),
        step_ms,
    }
}

pub fn calc_pb(solves: &[SolveEntry]) -> Option<u64> {
    solves.iter().map(|s| s.report.total_time_ms).min()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solve(id: usize, total_time_ms: u64) -> SolveEntry {
        SolveEntry {
            id,
            scramble: String::new(),
            report: SolveReportDto {
                total_time_ms,
                active_time_ms: total_time_ms,
                total_moves_htm: 0,
                tps_raw: 0.0,
                tps_effective: 0.0,
                phases: Vec::new(),
                pauses: Vec::new(),
                rotation_tax: RotationTaxDto::default(),
                lockups: Vec::new(),
                anomalies: Vec::new(),
            },
            raw_json: serde_json::Value::Null,
        }
    }

    #[test]
    fn adaptive_step_uses_half_second_when_range_within_4500ms() {
        let solves = vec![
            make_solve(1, 9500),
            make_solve(2, 10200),
            make_solve(3, 10490),
            make_solve(4, 12400),
        ];
        let dist = compute_time_distribution(&solves, DistributionStepMode::Auto);
        assert_eq!(dist.step_ms, 500);
        assert_eq!(dist.buckets[0].label, "9.5+");
        assert_eq!(dist.buckets[0].cum_label, "< 10.0");
        assert_eq!(dist.buckets[0].cum_count, 1);
        assert_eq!(dist.buckets[1].label, "10.0+");
        assert_eq!(dist.buckets[1].count, 2);
        assert_eq!(dist.buckets[1].cum_count, 3);
        let last = dist.buckets.last().expect("non-empty");
        assert_eq!(last.cum_count, 4);
        assert_eq!(last.total_count, 4);
    }

    #[test]
    fn adaptive_step_uses_one_second_between_4501_and_12000ms() {
        let solves = vec![make_solve(1, 9100), make_solve(2, 14500)];
        let dist = compute_time_distribution(&solves, DistributionStepMode::Auto);
        assert_eq!(dist.step_ms, 1000);
        assert_eq!(dist.buckets[0].label, "9+");
        assert_eq!(dist.buckets[0].cum_label, "< 10");
    }

    #[test]
    fn adaptive_step_uses_two_or_five_seconds_above_12000ms() {
        let solves_2s = vec![make_solve(1, 10000), make_solve(2, 23000)];
        let dist_2s = compute_time_distribution(&solves_2s, DistributionStepMode::Auto);
        assert_eq!(dist_2s.step_ms, 2000);

        let solves_5s = vec![make_solve(1, 10000), make_solve(2, 45000)];
        let dist_5s = compute_time_distribution(&solves_5s, DistributionStepMode::Auto);
        assert_eq!(dist_5s.step_ms, 5000);
    }

    #[test]
    fn filters_empty_buckets_and_handles_outliers() {
        let solves = vec![
            make_solve(1, 7000),
            make_solve(2, 10000),
            make_solve(3, 11000),
            make_solve(4, 12000),
        ];
        let dist = compute_time_distribution(&solves, DistributionStepMode::Auto);
        assert_eq!(dist.step_ms, 1000);
        let labels: Vec<&str> = dist.buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, vec!["7+", "10+", "11+", "12+"]);
        let b10 = &dist.buckets[1];
        assert_eq!(b10.label, "10+");
        assert_eq!(b10.count, 1);
        assert_eq!(b10.cum_count, 2);
    }
}
