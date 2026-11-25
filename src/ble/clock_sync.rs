use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ClockSync {
    base_cube_ms: Option<u64>,
    base_pc_instant: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignedTimestamp {
    pub cube_ms: u64,
    pub pc_instant: Instant,
    pub elapsed_solve_ms: u64,
    pub estimated_jitter_ms: i64,
}

impl Default for ClockSync {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockSync {
    pub fn new() -> Self {
        Self {
            base_cube_ms: None,
            base_pc_instant: None,
        }
    }

    /// Reinicia la sincronización (p. ej. al iniciar un nuevo solve o sesión).
    pub fn reset(&mut self) {
        self.base_cube_ms = None;
        self.base_pc_instant = None;
    }

    /// Registra la llegada de un paquete con su timestamp interno del cubo en ms
    /// y el instante de recepción local del PC.
    pub fn synchronize(&mut self, cube_ms: u64, receipt_instant: Instant) -> AlignedTimestamp {
        let (base_cube, base_instant) = match (self.base_cube_ms, self.base_pc_instant) {
            (Some(c), Some(i)) => (c, i),
            _ => {
                self.base_cube_ms = Some(cube_ms);
                self.base_pc_instant = Some(receipt_instant);
                return AlignedTimestamp {
                    cube_ms,
                    pc_instant: receipt_instant,
                    elapsed_solve_ms: 0,
                    estimated_jitter_ms: 0,
                };
            }
        };

        let elapsed_cube_ms = cube_ms.saturating_sub(base_cube);
        let aligned_pc_instant = base_instant + Duration::from_millis(elapsed_cube_ms);

        let elapsed_pc_ms = receipt_instant.duration_since(base_instant).as_millis() as u64;
        let estimated_jitter_ms = (elapsed_pc_ms as i64) - (elapsed_cube_ms as i64);

        AlignedTimestamp {
            cube_ms,
            pc_instant: aligned_pc_instant,
            elapsed_solve_ms: elapsed_cube_ms,
            estimated_jitter_ms,
        }
    }

    pub fn is_synchronized(&self) -> bool {
        self.base_cube_ms.is_some()
    }
}
