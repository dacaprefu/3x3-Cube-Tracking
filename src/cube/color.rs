use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    White,
    Yellow,
    Green,
    Blue,
    Orange,
    Red,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Face {
    U = 0,
    D = 1,
    F = 2,
    B = 3,
    L = 4,
    R = 5,
}

impl Face {
    pub const ALL: [Face; 6] = [
        Face::U,
        Face::D,
        Face::F,
        Face::B,
        Face::L,
        Face::R,
    ];
}
