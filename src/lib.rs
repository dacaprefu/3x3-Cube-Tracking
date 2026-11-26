pub mod analytics;
pub mod ble;
pub mod cfop;
pub mod cube;
pub mod gyro;
pub mod export;
pub mod ai_coach;

pub use ai_coach::analyze_solve_with_gemini;
pub use analytics::{
    phase_str, AnalyticsError, BlindRotation, LockupRecord, PauseRecord, PhaseMetrics, RotationTax,
    SolveAnalyzer, SolveReport,
};
pub use ble::{
    derive_key_iv, extract_mac_from_properties, scan_for_gan_cubes, AlignedTimestamp, BleError,
    ClockSync, DiscoveredGanCube, GanBleClient, GanCipher, GanEvent, GanFaceletsEvent,
    GanGyroEvent, GanMoveEvent, MoveDirection,
};
pub use cfop::{
    count_solved_f2l_pairs, is_cross_solved, is_cube_solved, is_oll_solved, CfopMetrics,
    CfopPhase, CfopTracker, PhaseSummary,
};
pub use cube::{
    parse_moves, Color, Corner, Cube, Edge, Face, Move, ParseMoveError, DEFAULT_CENTERS,
};
pub use gyro::{
    CubeOrientation, DetectedRotation, GyroError, GyroProcessor, RegripDetector, RegripEvent,
};
