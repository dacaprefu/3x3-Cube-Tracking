use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegripEvent {
    pub start_timestamp_ms: u64,
    pub end_timestamp_ms: u64,
    pub duration_ms: u64,
    pub peak_angle_deg: f32,
    pub peak_angular_speed: f32,
    pub tilt_axis: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RegripState {
    Idle,
    Tilting {
        start_ms: u64,
        peak_angle_rad: u32,       // Float bits para PartialEq seguro
        peak_angular_speed: u32,
    },
}

/// Detector de Regrips: detecta micro-inclinaciones rápidas previas a la ejecución
/// de algoritmos o giros, sin que ocurra giro de capa durante la inclinación.
#[derive(Debug, Clone)]
pub struct RegripDetector {
    state: RegripState,
    baseline_quat: Quat,
    last_move_timestamp_ms: Option<u64>,
    regrips: Vec<RegripEvent>,

    // Umbrales configurables
    pub min_angle_deg: f32,      // p. ej. 5.0 grados
    pub max_angle_deg: f32,      // p. ej. 35.0 grados (para no confundir con rotación de 90°)
    pub min_angular_speed: f32,  // p. ej. 1.2 rad/s
    pub settle_speed: f32,       // p. ej. 0.5 rad/s
}

impl Default for RegripDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl RegripDetector {
    pub fn new() -> Self {
        Self {
            state: RegripState::Idle,
            baseline_quat: Quat::IDENTITY,
            last_move_timestamp_ms: None,
            regrips: Vec::new(),
            min_angle_deg: 5.0,
            max_angle_deg: 35.0,
            min_angular_speed: 1.2,
            settle_speed: 0.5,
        }
    }

    pub fn regrips(&self) -> &[RegripEvent] {
        &self.regrips
    }

    pub fn regrip_count(&self) -> usize {
        self.regrips.len()
    }

    /// Notifica al detector de un giro de capa, invalidando cualquier inclinación en curso.
    pub fn on_move(&mut self, timestamp_ms: u64) {
        self.last_move_timestamp_ms = Some(timestamp_ms);
        self.state = RegripState::Idle;
    }

    /// Procesa una lectura del giroscopio.
    pub fn on_gyro(
        &mut self,
        timestamp_ms: u64,
        quat: Quat,
        ang_vel: Vec3,
    ) -> Option<RegripEvent> {
        let q_norm = quat.normalize();
        let speed = ang_vel.length();

        // Calcular ángulo de desviación respecto a la posición base de reposo
        // angle = 2 * arccos(|q1 . q2|)
        let dot = self.baseline_quat.dot(q_norm).abs().clamp(0.0, 1.0);
        let angle_rad = 2.0 * dot.acos();
        let angle_deg = angle_rad.to_degrees();

        let mut emitted_event = None;

        match self.state {
            RegripState::Idle => {
                // Si la velocidad angular supera el umbral y el ángulo se sitúa en el rango de micro-inclinación
                if speed >= self.min_angular_speed
                    && angle_deg >= self.min_angle_deg
                    && angle_deg <= self.max_angle_deg
                {
                    self.state = RegripState::Tilting {
                        start_ms: timestamp_ms,
                        peak_angle_rad: angle_rad.to_bits(),
                        peak_angular_speed: speed.to_bits(),
                    };
                } else if speed < self.settle_speed {
                    // Actualizar posición base de reposo suavemente
                    self.baseline_quat = q_norm;
                }
            }
            RegripState::Tilting {
                start_ms,
                peak_angle_rad: current_peak_rad_bits,
                peak_angular_speed: current_peak_speed_bits,
            } => {
                let current_peak_rad = f32::from_bits(current_peak_rad_bits);
                let current_peak_speed = f32::from_bits(current_peak_speed_bits);

                let new_peak_rad = current_peak_rad.max(angle_rad);
                let new_peak_speed = current_peak_speed.max(speed);

                // Si excede el ángulo máximo, es una rotación del cubo completa, no un regrip
                if angle_deg > self.max_angle_deg {
                    self.state = RegripState::Idle;
                    self.baseline_quat = q_norm;
                }
                // Si la velocidad se reduce por debajo del umbral de reposo, el regrip ha finalizado
                else if speed <= self.settle_speed {
                    let duration_ms = timestamp_ms.saturating_sub(start_ms);
                    let peak_deg = new_peak_rad.to_degrees();

                    // Comprobar que no haya ocurrido un giro de capa durante este intervalo
                    let move_during_tilt = self
                        .last_move_timestamp_ms
                        .map(|m_ts| m_ts >= start_ms && m_ts <= timestamp_ms)
                        .unwrap_or(false);

                    if !move_during_tilt && duration_ms >= 30 && duration_ms <= 500 {
                        let event = RegripEvent {
                            start_timestamp_ms: start_ms,
                            end_timestamp_ms: timestamp_ms,
                            duration_ms,
                            peak_angle_deg: peak_deg,
                            peak_angular_speed: new_peak_speed,
                            tilt_axis: ang_vel.normalize_or_zero(),
                        };
                        self.regrips.push(event.clone());
                        emitted_event = Some(event);
                    }

                    self.baseline_quat = q_norm;
                    self.state = RegripState::Idle;
                } else {
                    self.state = RegripState::Tilting {
                        start_ms,
                        peak_angle_rad: new_peak_rad.to_bits(),
                        peak_angular_speed: new_peak_speed.to_bits(),
                    };
                }
            }
        }

        emitted_event
    }
}
