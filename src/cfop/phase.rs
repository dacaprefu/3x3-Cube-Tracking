use serde::{Deserialize, Serialize};

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CfopPhase {
    Scramble,
    Inspection,
    Cross,
    F2L_1,
    F2L_2,
    F2L_3,
    F2L_4,
    OLL,
    PLL,
    Solved,
}

impl std::fmt::Display for CfopPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfopPhase::Scramble => write!(f, "Scramble"),
            CfopPhase::Inspection => write!(f, "Inspection"),
            CfopPhase::Cross => write!(f, "Cross"),
            CfopPhase::F2L_1 => write!(f, "F2L 1"),
            CfopPhase::F2L_2 => write!(f, "F2L 2"),
            CfopPhase::F2L_3 => write!(f, "F2L 3"),
            CfopPhase::F2L_4 => write!(f, "F2L 4"),
            CfopPhase::OLL => write!(f, "OLL"),
            CfopPhase::PLL => write!(f, "PLL"),
            CfopPhase::Solved => write!(f, "Solved"),
        }
    }
}
