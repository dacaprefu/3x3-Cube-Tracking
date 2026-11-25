use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
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
