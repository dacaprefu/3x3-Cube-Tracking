use crate::cube::color::Face;
use crate::cube::moves::Move;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::FRAC_PI_2;

/// Orientación espacial discreta del cubo (una de las 24 orientaciones posibles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CubeOrientation {
    pub up: Face,
    pub front: Face,
}

impl Default for CubeOrientation {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl CubeOrientation {
    pub const IDENTITY: Self = Self {
        up: Face::U,
        front: Face::F,
    };

    pub fn new(up: Face, front: Face) -> Option<Self> {
        if up == front || up == opposite_face(front) {
            None
        } else {
            Some(Self { up, front })
        }
    }

    pub fn right(&self) -> Face {
        match (self.up, self.front) {
            (Face::U, Face::F) => Face::R,
            (Face::U, Face::R) => Face::B,
            (Face::U, Face::B) => Face::L,
            (Face::U, Face::L) => Face::F,

            (Face::D, Face::F) => Face::L,
            (Face::D, Face::L) => Face::B,
            (Face::D, Face::B) => Face::R,
            (Face::D, Face::R) => Face::F,

            (Face::F, Face::U) => Face::L,
            (Face::F, Face::L) => Face::D,
            (Face::F, Face::D) => Face::R,
            (Face::F, Face::R) => Face::U,

            (Face::B, Face::U) => Face::R,
            (Face::B, Face::R) => Face::D,
            (Face::B, Face::D) => Face::L,
            (Face::B, Face::L) => Face::U,

            (Face::L, Face::U) => Face::F,
            (Face::L, Face::F) => Face::D,
            (Face::L, Face::D) => Face::B,
            (Face::L, Face::B) => Face::U,

            (Face::R, Face::U) => Face::B,
            (Face::R, Face::B) => Face::D,
            (Face::R, Face::D) => Face::F,
            (Face::R, Face::F) => Face::U,

            _ => Face::R,
        }
    }

    pub fn left(&self) -> Face {
        opposite_face(self.right())
    }

    pub fn back(&self) -> Face {
        opposite_face(self.front)
    }

    pub fn down(&self) -> Face {
        opposite_face(self.up)
    }

    /// Aplica una rotación del cubo a la orientación actual.
    pub fn apply_rotation(&self, r: Move) -> Self {
        match r {
            Move::Y => Self {
                up: self.up,
                front: self.right(),
            },
            Move::YPrime => Self {
                up: self.up,
                front: self.left(),
            },
            Move::Y2 => self.apply_rotation(Move::Y).apply_rotation(Move::Y),

            Move::X => Self {
                up: self.front,
                front: self.down(),
            },
            Move::XPrime => Self {
                up: self.back(),
                front: self.up,
            },
            Move::X2 => self.apply_rotation(Move::X).apply_rotation(Move::X),

            Move::Z => Self {
                up: self.left(),
                front: self.front,
            },
            Move::ZPrime => Self {
                up: self.right(),
                front: self.front,
            },
            Move::Z2 => self.apply_rotation(Move::Z).apply_rotation(Move::Z),

            _ => *self,
        }
    }

    /// Mapea un giro que el usuario hace desde su perspectiva (la cara que ve frente a él)
    /// a la cara correspondiente en el cubo interno.
    pub fn perspective_to_cube_face(&self, user_face: Face) -> Face {
        match user_face {
            Face::U => self.up,
            Face::D => self.down(),
            Face::F => self.front,
            Face::B => self.back(),
            Face::L => self.left(),
            Face::R => self.right(),
        }
    }

    /// Mapea un movimiento en perspectiva de usuario al movimiento correspondiente del cubo interno.
    pub fn perspective_to_cube_move(&self, user_move: Move) -> Move {
        let (user_face, is_prime, is_double) = match user_move {
            Move::U => (Face::U, false, false),
            Move::UPrime => (Face::U, true, false),
            Move::U2 => (Face::U, false, true),

            Move::D => (Face::D, false, false),
            Move::DPrime => (Face::D, true, false),
            Move::D2 => (Face::D, false, true),

            Move::F => (Face::F, false, false),
            Move::FPrime => (Face::F, true, false),
            Move::F2 => (Face::F, false, true),

            Move::B => (Face::B, false, false),
            Move::BPrime => (Face::B, true, false),
            Move::B2 => (Face::B, false, true),

            Move::L => (Face::L, false, false),
            Move::LPrime => (Face::L, true, false),
            Move::L2 => (Face::L, false, true),

            Move::R => (Face::R, false, false),
            Move::RPrime => (Face::R, true, false),
            Move::R2 => (Face::R, false, true),

            other => return other,
        };

        let target_face = self.perspective_to_cube_face(user_face);
        match (target_face, is_prime, is_double) {
            (Face::U, false, false) => Move::U,
            (Face::U, true, false) => Move::UPrime,
            (Face::U, false, true) => Move::U2,

            (Face::D, false, false) => Move::D,
            (Face::D, true, false) => Move::DPrime,
            (Face::D, false, true) => Move::D2,

            (Face::F, false, false) => Move::F,
            (Face::F, true, false) => Move::FPrime,
            (Face::F, false, true) => Move::F2,

            (Face::B, false, false) => Move::B,
            (Face::B, true, false) => Move::BPrime,
            (Face::B, false, true) => Move::B2,

            (Face::L, false, false) => Move::L,
            (Face::L, true, false) => Move::LPrime,
            (Face::L, false, true) => Move::L2,

            (Face::R, false, false) => Move::R,
            (Face::R, true, false) => Move::RPrime,
            (Face::R, false, true) => Move::R2,
            _ => Move::U,
        }
    }
}

pub fn opposite_face(face: Face) -> Face {
    match face {
        Face::U => Face::D,
        Face::D => Face::U,
        Face::F => Face::B,
        Face::B => Face::F,
        Face::L => Face::R,
        Face::R => Face::L,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DetectedRotation {
    pub rotation: Move,
    pub previous_orientation: CubeOrientation,
    pub new_orientation: CubeOrientation,
    pub timestamp_ms: u64,
}

/// Procesador de cuaterniones del giroscopio para detección de rotaciones completas (x, y, z).
#[derive(Debug, Clone)]
pub struct GyroProcessor {
    current_orientation: CubeOrientation,
    reference_quat: Quat,
    all_orientations: Vec<(CubeOrientation, Quat)>,
}

impl Default for GyroProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GyroProcessor {
    pub fn new() -> Self {
        let all = generate_all_orientations();
        Self {
            current_orientation: CubeOrientation::IDENTITY,
            reference_quat: Quat::IDENTITY,
            all_orientations: all,
        }
    }

    pub fn current_orientation(&self) -> CubeOrientation {
        self.current_orientation
    }

    pub fn reset_orientation(&mut self, orientation: CubeOrientation) {
        self.current_orientation = orientation;
        if let Some((_, q)) = self.all_orientations.iter().find(|(o, _)| *o == orientation) {
            self.reference_quat = *q;
        }
    }

    /// Procesa una lectura de cuaternión y detecta si se ha completado una rotación de 90° o 180°.
    pub fn process_quaternion(&mut self, q: Quat, timestamp_ms: u64) -> Option<DetectedRotation> {
        let q_norm = q.normalize();

        // Encontrar la orientación nominal más cercana
        let mut best_orientation = self.current_orientation;
        let mut max_dot = -1.0f32;

        for (orient, nominal_q) in &self.all_orientations {
            let dot = q_norm.dot(*nominal_q).abs();
            if dot > max_dot {
                max_dot = dot;
                best_orientation = *orient;
            }
        }

        // Si la orientación más cercana es diferente a la actual y tiene alta confianza
        // cos(35°) ≈ 0.819 -> si dot > 0.82 estamos dentro del umbral de atracción de esa orientación
        if best_orientation != self.current_orientation && max_dot > 0.82 {
            let prev = self.current_orientation;
            let rot = find_transition_rotation(prev, best_orientation)?;

            self.current_orientation = best_orientation;
            return Some(DetectedRotation {
                rotation: rot,
                previous_orientation: prev,
                new_orientation: best_orientation,
                timestamp_ms,
            });
        }

        None
    }
}

fn find_transition_rotation(from: CubeOrientation, to: CubeOrientation) -> Option<Move> {
    let candidate_moves = [
        Move::Y,
        Move::YPrime,
        Move::Y2,
        Move::X,
        Move::XPrime,
        Move::X2,
        Move::Z,
        Move::ZPrime,
        Move::Z2,
    ];

    for m in candidate_moves {
        if from.apply_rotation(m) == to {
            return Some(m);
        }
    }
    None
}

/// Genera las 24 orientaciones canónicas y sus cuaterniones asociados en el espacio.
fn generate_all_orientations() -> Vec<(CubeOrientation, Quat)> {
    let mut list: Vec<(CubeOrientation, Quat)> = Vec::with_capacity(24);
    let mut queue = std::collections::VecDeque::new();

    let q_y = Quat::from_axis_angle(Vec3::Y, -FRAC_PI_2);
    let q_x = Quat::from_axis_angle(Vec3::X, FRAC_PI_2);
    let q_z = Quat::from_axis_angle(Vec3::Z, -FRAC_PI_2);

    let start_or = CubeOrientation::IDENTITY;
    let start_q = Quat::IDENTITY;

    list.push((start_or, start_q));
    queue.push_back((start_or, start_q));

    let moves = [
        (Move::X, q_x),
        (Move::Y, q_y),
        (Move::Z, q_z),
    ];

    while let Some((or, q)) = queue.pop_front() {
        for &(m, q_rot) in &moves {
            let next_or = or.apply_rotation(m);
            let next_q = (q_rot * q).normalize();

            if !list.iter().any(|(existing_or, _)| *existing_or == next_or) {
                list.push((next_or, next_q));
                queue.push_back((next_or, next_q));
            }
        }
    }

    list
}
