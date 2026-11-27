use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParseMoveError {
    #[error("Movimiento desconocido o inválido: '{0}'")]
    InvalidMove(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Move {
    U,
    UPrime,
    U2,
    D,
    DPrime,
    D2,
    L,
    LPrime,
    L2,
    R,
    RPrime,
    R2,
    F,
    FPrime,
    F2,
    B,
    BPrime,
    B2,
    X,
    XPrime,
    X2,
    Y,
    YPrime,
    Y2,
    Z,
    ZPrime,
    Z2,
}

impl Move {
    pub fn is_auf(&self) -> bool {
        matches!(self, Move::U | Move::UPrime | Move::U2)
    }

    pub fn is_rotation(&self) -> bool {
        matches!(
            self,
            Move::X
                | Move::XPrime
                | Move::X2
                | Move::Y
                | Move::YPrime
                | Move::Y2
                | Move::Z
                | Move::ZPrime
                | Move::Z2
        )
    }

    pub fn inverse(&self) -> Self {
        match self {
            Move::U => Move::UPrime,
            Move::UPrime => Move::U,
            Move::U2 => Move::U2,
            Move::D => Move::DPrime,
            Move::DPrime => Move::D,
            Move::D2 => Move::D2,
            Move::L => Move::LPrime,
            Move::LPrime => Move::L,
            Move::L2 => Move::L2,
            Move::R => Move::RPrime,
            Move::RPrime => Move::R,
            Move::R2 => Move::R2,
            Move::F => Move::FPrime,
            Move::FPrime => Move::F,
            Move::F2 => Move::F2,
            Move::B => Move::BPrime,
            Move::BPrime => Move::B,
            Move::B2 => Move::B2,
            Move::X => Move::XPrime,
            Move::XPrime => Move::X,
            Move::X2 => Move::X2,
            Move::Y => Move::YPrime,
            Move::YPrime => Move::Y,
            Move::Y2 => Move::Y2,
            Move::Z => Move::ZPrime,
            Move::ZPrime => Move::Z,
            Move::Z2 => Move::Z2,
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Move::U => "U",
            Move::UPrime => "U'",
            Move::U2 => "U2",
            Move::D => "D",
            Move::DPrime => "D'",
            Move::D2 => "D2",
            Move::L => "L",
            Move::LPrime => "L'",
            Move::L2 => "L2",
            Move::R => "R",
            Move::RPrime => "R'",
            Move::R2 => "R2",
            Move::F => "F",
            Move::FPrime => "F'",
            Move::F2 => "F2",
            Move::B => "B",
            Move::BPrime => "B'",
            Move::B2 => "B2",
            Move::X => "x",
            Move::XPrime => "x'",
            Move::X2 => "x2",
            Move::Y => "y",
            Move::YPrime => "y'",
            Move::Y2 => "y2",
            Move::Z => "z",
            Move::ZPrime => "z'",
            Move::Z2 => "z2",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        match trimmed {
            "U" => Ok(Move::U),
            "U'" => Ok(Move::UPrime),
            "U2" => Ok(Move::U2),
            "D" => Ok(Move::D),
            "D'" => Ok(Move::DPrime),
            "D2" => Ok(Move::D2),
            "L" => Ok(Move::L),
            "L'" => Ok(Move::LPrime),
            "L2" => Ok(Move::L2),
            "R" => Ok(Move::R),
            "R'" => Ok(Move::RPrime),
            "R2" => Ok(Move::R2),
            "F" => Ok(Move::F),
            "F'" => Ok(Move::FPrime),
            "F2" => Ok(Move::F2),
            "B" => Ok(Move::B),
            "B'" => Ok(Move::BPrime),
            "B2" => Ok(Move::B2),
            "x" | "X" => Ok(Move::X),
            "x'" | "X'" => Ok(Move::XPrime),
            "x2" | "X2" => Ok(Move::X2),
            "y" | "Y" => Ok(Move::Y),
            "y'" | "Y'" => Ok(Move::YPrime),
            "y2" | "Y2" => Ok(Move::Y2),
            "z" | "Z" => Ok(Move::Z),
            "z'" | "Z'" => Ok(Move::ZPrime),
            "z2" | "Z2" => Ok(Move::Z2),
            _ => Err(ParseMoveError::InvalidMove(s.to_string())),
        }
    }
}

pub fn parse_moves(s: &str) -> Result<Vec<Move>, ParseMoveError> {
    s.split_whitespace()
        .map(|token| token.parse::<Move>())
        .collect()
}

static SCRAMBLE_SEED: AtomicU64 = AtomicU64::new(0);

fn next_random_u64() -> u64 {
    let time_entropy = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e37_79b9_7f4a_7c15);

    let prev = SCRAMBLE_SEED.fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed);
    let mut x = prev ^ time_entropy.wrapping_add(0xbf58_476d_1ce4_e5b9);
    if x == 0 {
        x = 0x94d0_49bb_1331_11eb;
    }
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    x.wrapping_mul(0x2545_f491_4f6c_dd1d)
}

/// Genera un scramble WCA estándar para 3x3 sin movimientos consecutivos en la misma cara
/// ni secuencias redundantes en el mismo eje (p. ej. `R L R'`, `U D U`, `F B F2`).
pub fn generate_wca_scramble(length: usize) -> Vec<String> {
    const FACES: [&str; 6] = ["R", "L", "U", "D", "F", "B"];
    const MODIFIERS: [&str; 3] = ["", "'", "2"];

    let mut result = Vec::with_capacity(length);
    let mut last_face: Option<usize> = None;
    let mut second_last_face: Option<usize> = None;

    while result.len() < length {
        let face_idx = (next_random_u64() % 6) as usize;
        let axis = face_idx / 2;

        if Some(face_idx) == last_face {
            continue;
        }
        if let (Some(lf), Some(slf)) = (last_face, second_last_face) {
            if axis == lf / 2 && axis == slf / 2 {
                continue;
            }
        }

        let mod_idx = (next_random_u64() % 3) as usize;
        result.push(format!("{}{}", FACES[face_idx], MODIFIERS[mod_idx]));
        second_last_face = last_face;
        last_face = Some(face_idx);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wca_scramble_rules() {
        let scramble = generate_wca_scramble(20);
        assert_eq!(scramble.len(), 20);
        let parsed = parse_moves(&scramble.join(" ")).expect("scramble válido");
        assert_eq!(parsed.len(), 20);

        let axis_of = |s: &str| -> u8 {
            match s.chars().next() {
                Some('R') | Some('L') => 0,
                Some('U') | Some('D') => 1,
                _ => 2,
            }
        };
        let face_of = |s: &str| -> char { s.chars().next().unwrap_or(' ') };

        for i in 1..scramble.len() {
            assert_ne!(face_of(&scramble[i]), face_of(&scramble[i - 1]));
            if i >= 2 {
                let a0 = axis_of(&scramble[i - 2]);
                let a1 = axis_of(&scramble[i - 1]);
                let a2 = axis_of(&scramble[i]);
                assert!(!(a0 == a1 && a1 == a2), "Eje redundante en {:?}", &scramble[i - 2..=i]);
            }
        }
    }
}
