use crate::cube::color::{Color, Face};
use crate::cube::moves::Move;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Corner {
    pub piece: u8,       // 0..8
    pub orientation: u8, // 0, 1, 2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edge {
    pub piece: u8,       // 0..12
    pub orientation: u8, // 0, 1
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cube {
    pub corners: [Corner; 8],
    pub edges: [Edge; 12],
    pub centers: [Color; 6], // Index by Face as usize: U, D, F, B, L, R
}

// Corner pieces solved colors: [U/D, Face1, Face2] clockwise
const CORNER_COLORS: [[Color; 3]; 8] = [
    [Color::White, Color::Green, Color::Orange], // 0: UFL
    [Color::White, Color::Red, Color::Green],    // 1: UFR
    [Color::White, Color::Blue, Color::Red],     // 2: UBR
    [Color::White, Color::Orange, Color::Blue],   // 3: UBL
    [Color::Yellow, Color::Orange, Color::Green], // 4: DFL
    [Color::Yellow, Color::Green, Color::Red],    // 5: DFR
    [Color::Yellow, Color::Red, Color::Blue],     // 6: DBR
    [Color::Yellow, Color::Blue, Color::Orange],  // 7: DBL
];

// Corner slots sticker indices: [U/D, Face1, Face2]
// Face offset = Face as usize * 9
const CORNER_SLOTS: [[usize; 3]; 8] = [
    [0 * 9 + 6, 2 * 9 + 0, 4 * 9 + 2], // 0: UFL (U6, F0, L2)
    [0 * 9 + 8, 5 * 9 + 0, 2 * 9 + 2], // 1: UFR (U8, R0, F2)
    [0 * 9 + 2, 3 * 9 + 0, 5 * 9 + 2], // 2: UBR (U2, B0, R2)
    [0 * 9 + 0, 4 * 9 + 0, 3 * 9 + 2], // 3: UBL (U0, L0, B2)
    [1 * 9 + 0, 4 * 9 + 8, 2 * 9 + 6], // 4: DFL (D0, L8, F6)
    [1 * 9 + 2, 2 * 9 + 8, 5 * 9 + 6], // 5: DFR (D2, F8, R6)
    [1 * 9 + 8, 5 * 9 + 8, 3 * 9 + 6], // 6: DBR (D8, R8, B6)
    [1 * 9 + 6, 3 * 9 + 8, 4 * 9 + 6], // 7: DBL (D6, B8, L6)
];

// Edge pieces solved colors: [Primary, Secondary]
const EDGE_COLORS: [[Color; 2]; 12] = [
    [Color::White, Color::Green],  // 0: UF
    [Color::White, Color::Red],    // 1: UR
    [Color::White, Color::Blue],   // 2: UB
    [Color::White, Color::Orange], // 3: UL
    [Color::Green, Color::Orange], // 4: FL
    [Color::Green, Color::Red],    // 5: FR
    [Color::Blue, Color::Red],     // 6: BR
    [Color::Blue, Color::Orange],  // 7: BL
    [Color::Yellow, Color::Green], // 8: DF
    [Color::Yellow, Color::Red],   // 9: DR
    [Color::Yellow, Color::Blue],  // 10: DB
    [Color::Yellow, Color::Orange],// 11: DL
];

// Edge slots sticker indices: [Primary, Secondary]
const EDGE_SLOTS: [[usize; 2]; 12] = [
    [0 * 9 + 7, 2 * 9 + 1], // 0: UF (U7, F1)
    [0 * 9 + 5, 5 * 9 + 1], // 1: UR (U5, R1)
    [0 * 9 + 1, 3 * 9 + 1], // 2: UB (U1, B1)
    [0 * 9 + 3, 4 * 9 + 1], // 3: UL (U3, L1)
    [2 * 9 + 3, 4 * 9 + 5], // 4: FL (F3, L5)
    [2 * 9 + 5, 5 * 9 + 3], // 5: FR (F5, R3)
    [3 * 9 + 3, 5 * 9 + 5], // 6: BR (B3, R5)
    [3 * 9 + 5, 4 * 9 + 3], // 7: BL (B5, L3)
    [1 * 9 + 1, 2 * 9 + 7], // 8: DF (D1, F7)
    [1 * 9 + 5, 5 * 9 + 7], // 9: DR (D5, R7)
    [1 * 9 + 7, 3 * 9 + 7], // 10: DB (D7, B7)
    [1 * 9 + 3, 4 * 9 + 7], // 11: DL (D3, L7)
];

const CENTER_SLOTS: [usize; 6] = [
    0 * 9 + 4, // U
    1 * 9 + 4, // D
    2 * 9 + 4, // F
    3 * 9 + 4, // B
    4 * 9 + 4, // L
    5 * 9 + 4, // R
];

pub const DEFAULT_CENTERS: [Color; 6] = [
    Color::White,  // U
    Color::Yellow, // D
    Color::Green,  // F
    Color::Blue,   // B
    Color::Orange, // L
    Color::Red,    // R
];

impl Default for Cube {
    fn default() -> Self {
        Self::new()
    }
}

impl Cube {
    pub fn new() -> Self {
        let mut corners = [Corner { piece: 0, orientation: 0 }; 8];
        for i in 0..8 {
            corners[i] = Corner {
                piece: i as u8,
                orientation: 0,
            };
        }

        let mut edges = [Edge { piece: 0, orientation: 0 }; 12];
        for i in 0..12 {
            edges[i] = Edge {
                piece: i as u8,
                orientation: 0,
            };
        }

        Self {
            corners,
            edges,
            centers: DEFAULT_CENTERS,
        }
    }

    /// Proyecta el estado matemático a 54 stickers ordenados por caras (U, D, F, B, L, R)
    /// con 9 stickers cada una.
    pub fn to_stickers(&self) -> [Color; 54] {
        let mut stickers = [Color::White; 54];

        // Centers
        for (face_idx, &center_slot) in CENTER_SLOTS.iter().enumerate() {
            stickers[center_slot] = self.centers[face_idx];
        }

        // Corners
        for slot in 0..8 {
            let corner = self.corners[slot];
            let p = corner.piece as usize;
            let ori = corner.orientation as usize;
            let colors = CORNER_COLORS[p];
            let slot_stickers = CORNER_SLOTS[slot];

            for c in 0..3 {
                let target_slot_idx = slot_stickers[(c + ori) % 3];
                stickers[target_slot_idx] = colors[c];
            }
        }

        // Edges
        for slot in 0..12 {
            let edge = self.edges[slot];
            let p = edge.piece as usize;
            let ori = edge.orientation as usize;
            let colors = EDGE_COLORS[p];
            let slot_stickers = EDGE_SLOTS[slot];

            for c in 0..2 {
                let target_slot_idx = slot_stickers[(c + ori) % 2];
                stickers[target_slot_idx] = colors[c];
            }
        }

        stickers
    }

    /// Reconstruye el cubo a partir de los 54 stickers.
    pub fn from_stickers(stickers: &[Color; 54]) -> Self {
        let mut centers = DEFAULT_CENTERS;
        for (face_idx, &center_slot) in CENTER_SLOTS.iter().enumerate() {
            centers[face_idx] = stickers[center_slot];
        }

        let mut corners = [Corner { piece: 0, orientation: 0 }; 8];
        for slot in 0..8 {
            let slot_stickers = CORNER_SLOTS[slot];
            let c0 = stickers[slot_stickers[0]];
            let c1 = stickers[slot_stickers[1]];
            let c2 = stickers[slot_stickers[2]];

            // Encontrar la pieza correspondiente
            let mut found_piece = 0;
            let mut found_ori = 0;

            for (p_idx, p_colors) in CORNER_COLORS.iter().enumerate() {
                let has_c0 = p_colors.contains(&c0);
                let has_c1 = p_colors.contains(&c1);
                let has_c2 = p_colors.contains(&c2);

                if has_c0 && has_c1 && has_c2 {
                    found_piece = p_idx as u8;
                    // Orientation: dónde está el color U/D (p_colors[0])
                    let u_d_color = p_colors[0];
                    if c0 == u_d_color {
                        found_ori = 0;
                    } else if c1 == u_d_color {
                        found_ori = 1;
                    } else {
                        found_ori = 2;
                    }
                    break;
                }
            }

            corners[slot] = Corner {
                piece: found_piece,
                orientation: found_ori,
            };
        }

        let mut edges = [Edge { piece: 0, orientation: 0 }; 12];
        for slot in 0..12 {
            let slot_stickers = EDGE_SLOTS[slot];
            let c0 = stickers[slot_stickers[0]];
            let c1 = stickers[slot_stickers[1]];

            let mut found_piece = 0;
            let mut found_ori = 0;

            for (p_idx, p_colors) in EDGE_COLORS.iter().enumerate() {
                let has_c0 = p_colors.contains(&c0);
                let has_c1 = p_colors.contains(&c1);

                if has_c0 && has_c1 {
                    found_piece = p_idx as u8;
                    let primary_color = p_colors[0];
                    if c0 == primary_color {
                        found_ori = 0;
                    } else {
                        found_ori = 1;
                    }
                    break;
                }
            }

            edges[slot] = Edge {
                piece: found_piece,
                orientation: found_ori,
            };
        }

        Self {
            corners,
            edges,
            centers,
        }
    }

    /// Verifica si el cubo está completamente resuelto en su orientación estándar.
    pub fn is_solved(&self) -> bool {
        if self.centers != DEFAULT_CENTERS {
            return false;
        }
        for (i, c) in self.corners.iter().enumerate() {
            if c.piece != i as u8 || c.orientation != 0 {
                return false;
            }
        }
        for (i, e) in self.edges.iter().enumerate() {
            if e.piece != i as u8 || e.orientation != 0 {
                return false;
            }
        }
        true
    }

    /// Aplica un movimiento (`U, D, L, R, F, B`, primas, dobles) o rotación (`x, y, z`).
    pub fn apply_move(&mut self, m: Move) {
        let mut stickers = self.to_stickers();

        match m {
            Move::U => apply_u_cw(&mut stickers),
            Move::UPrime => {
                apply_u_cw(&mut stickers);
                apply_u_cw(&mut stickers);
                apply_u_cw(&mut stickers);
            }
            Move::U2 => {
                apply_u_cw(&mut stickers);
                apply_u_cw(&mut stickers);
            }

            Move::D => apply_d_cw(&mut stickers),
            Move::DPrime => {
                apply_d_cw(&mut stickers);
                apply_d_cw(&mut stickers);
                apply_d_cw(&mut stickers);
            }
            Move::D2 => {
                apply_d_cw(&mut stickers);
                apply_d_cw(&mut stickers);
            }

            Move::F => apply_f_cw(&mut stickers),
            Move::FPrime => {
                apply_f_cw(&mut stickers);
                apply_f_cw(&mut stickers);
                apply_f_cw(&mut stickers);
            }
            Move::F2 => {
                apply_f_cw(&mut stickers);
                apply_f_cw(&mut stickers);
            }

            Move::B => apply_b_cw(&mut stickers),
            Move::BPrime => {
                apply_b_cw(&mut stickers);
                apply_b_cw(&mut stickers);
                apply_b_cw(&mut stickers);
            }
            Move::B2 => {
                apply_b_cw(&mut stickers);
                apply_b_cw(&mut stickers);
            }

            Move::L => apply_l_cw(&mut stickers),
            Move::LPrime => {
                apply_l_cw(&mut stickers);
                apply_l_cw(&mut stickers);
                apply_l_cw(&mut stickers);
            }
            Move::L2 => {
                apply_l_cw(&mut stickers);
                apply_l_cw(&mut stickers);
            }

            Move::R => apply_r_cw(&mut stickers),
            Move::RPrime => {
                apply_r_cw(&mut stickers);
                apply_r_cw(&mut stickers);
                apply_r_cw(&mut stickers);
            }
            Move::R2 => {
                apply_r_cw(&mut stickers);
                apply_r_cw(&mut stickers);
            }

            Move::Y => apply_y_rot(&mut stickers),
            Move::YPrime => {
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
            }
            Move::Y2 => {
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
            }

            Move::Z => apply_z_rot(&mut stickers),
            Move::ZPrime => {
                apply_z_rot(&mut stickers);
                apply_z_rot(&mut stickers);
                apply_z_rot(&mut stickers);
            }
            Move::Z2 => {
                apply_z_rot(&mut stickers);
                apply_z_rot(&mut stickers);
            }

            // x = y * z * y'
            Move::X => {
                apply_y_rot(&mut stickers);
                apply_z_rot(&mut stickers);
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
            }
            Move::XPrime => {
                apply_y_rot(&mut stickers);
                apply_z_rot(&mut stickers);
                apply_z_rot(&mut stickers);
                apply_z_rot(&mut stickers);
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
                apply_y_rot(&mut stickers);
            }
            Move::X2 => {
                self.apply_move(Move::X);
                self.apply_move(Move::X);
                return;
            }
        }

        *self = Self::from_stickers(&stickers);
    }

    /// Aplica una secuencia de movimientos.
    pub fn apply_moves(&mut self, moves: &[Move]) {
        for &m in moves {
            self.apply_move(m);
        }
    }

    /// Comprueba si el estado actual coincide exactamente con el resultado de aplicar
    /// un scramble desde el estado resuelto.
    pub fn matches_scramble(&self, scramble: &[Move]) -> bool {
        let mut expected = Cube::new();
        expected.apply_moves(scramble);
        self == &expected
    }
}

// Rotación en sentido horario de una cara de 9 stickers
fn rotate_face_cw(stickers: &mut [Color; 54], face: Face) {
    let base = (face as usize) * 9;
    let old = [
        stickers[base],
        stickers[base + 1],
        stickers[base + 2],
        stickers[base + 3],
        stickers[base + 4],
        stickers[base + 5],
        stickers[base + 6],
        stickers[base + 7],
        stickers[base + 8],
    ];

    stickers[base + 0] = old[6];
    stickers[base + 1] = old[3];
    stickers[base + 2] = old[0];
    stickers[base + 3] = old[7];
    stickers[base + 4] = old[4];
    stickers[base + 5] = old[1];
    stickers[base + 6] = old[8];
    stickers[base + 7] = old[5];
    stickers[base + 8] = old[2];
}

fn rotate_face_ccw(stickers: &mut [Color; 54], face: Face) {
    rotate_face_cw(stickers, face);
    rotate_face_cw(stickers, face);
    rotate_face_cw(stickers, face);
}

fn apply_u_cw(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::U);

    let old = *stickers;
    // F -> L -> B -> R -> F (row 0: 0, 1, 2)
    let f_base = (Face::F as usize) * 9;
    let l_base = (Face::L as usize) * 9;
    let b_base = (Face::B as usize) * 9;
    let r_base = (Face::R as usize) * 9;

    stickers[l_base + 0] = old[f_base + 0];
    stickers[l_base + 1] = old[f_base + 1];
    stickers[l_base + 2] = old[f_base + 2];

    stickers[b_base + 0] = old[l_base + 0];
    stickers[b_base + 1] = old[l_base + 1];
    stickers[b_base + 2] = old[l_base + 2];

    stickers[r_base + 0] = old[b_base + 0];
    stickers[r_base + 1] = old[b_base + 1];
    stickers[r_base + 2] = old[b_base + 2];

    stickers[f_base + 0] = old[r_base + 0];
    stickers[f_base + 1] = old[r_base + 1];
    stickers[f_base + 2] = old[r_base + 2];
}

fn apply_d_cw(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::D);

    let old = *stickers;
    // F -> R -> B -> L -> F (row 2: 6, 7, 8)
    let f_base = (Face::F as usize) * 9;
    let r_base = (Face::R as usize) * 9;
    let b_base = (Face::B as usize) * 9;
    let l_base = (Face::L as usize) * 9;

    stickers[r_base + 6] = old[f_base + 6];
    stickers[r_base + 7] = old[f_base + 7];
    stickers[r_base + 8] = old[f_base + 8];

    stickers[b_base + 6] = old[r_base + 6];
    stickers[b_base + 7] = old[r_base + 7];
    stickers[b_base + 8] = old[r_base + 8];

    stickers[l_base + 6] = old[b_base + 6];
    stickers[l_base + 7] = old[b_base + 7];
    stickers[l_base + 8] = old[b_base + 8];

    stickers[f_base + 6] = old[l_base + 6];
    stickers[f_base + 7] = old[l_base + 7];
    stickers[f_base + 8] = old[l_base + 8];
}

fn apply_f_cw(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::F);

    let old = *stickers;
    let u_base = (Face::U as usize) * 9;
    let r_base = (Face::R as usize) * 9;
    let d_base = (Face::D as usize) * 9;
    let l_base = (Face::L as usize) * 9;

    // U(6,7,8) -> R(0,3,6)
    stickers[r_base + 0] = old[u_base + 6];
    stickers[r_base + 3] = old[u_base + 7];
    stickers[r_base + 6] = old[u_base + 8];

    // R(0,3,6) -> D(2,1,0)
    stickers[d_base + 2] = old[r_base + 0];
    stickers[d_base + 1] = old[r_base + 3];
    stickers[d_base + 0] = old[r_base + 6];

    // D(0,1,2) -> L(2,5,8)
    stickers[l_base + 2] = old[d_base + 0];
    stickers[l_base + 5] = old[d_base + 1];
    stickers[l_base + 8] = old[d_base + 2];

    // L(2,5,8) -> U(8,7,6)
    stickers[u_base + 8] = old[l_base + 2];
    stickers[u_base + 7] = old[l_base + 5];
    stickers[u_base + 6] = old[l_base + 8];
}

fn apply_b_cw(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::B);

    let old = *stickers;
    let u_base = (Face::U as usize) * 9;
    let l_base = (Face::L as usize) * 9;
    let d_base = (Face::D as usize) * 9;
    let r_base = (Face::R as usize) * 9;

    // U(2,1,0) -> L(0,3,6)
    stickers[l_base + 0] = old[u_base + 2];
    stickers[l_base + 3] = old[u_base + 1];
    stickers[l_base + 6] = old[u_base + 0];

    // L(0,3,6) -> D(6,7,8)
    stickers[d_base + 6] = old[l_base + 0];
    stickers[d_base + 7] = old[l_base + 3];
    stickers[d_base + 8] = old[l_base + 6];

    // D(8,7,6) -> R(2,5,8)
    stickers[r_base + 2] = old[d_base + 8];
    stickers[r_base + 5] = old[d_base + 7];
    stickers[r_base + 8] = old[d_base + 6];

    // R(2,5,8) -> U(0,1,2)
    stickers[u_base + 0] = old[r_base + 2];
    stickers[u_base + 1] = old[r_base + 5];
    stickers[u_base + 2] = old[r_base + 8];
}

fn apply_l_cw(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::L);

    let old = *stickers;
    let u_base = (Face::U as usize) * 9;
    let f_base = (Face::F as usize) * 9;
    let d_base = (Face::D as usize) * 9;
    let b_base = (Face::B as usize) * 9;

    // U(0,3,6) -> F(0,3,6)
    stickers[f_base + 0] = old[u_base + 0];
    stickers[f_base + 3] = old[u_base + 3];
    stickers[f_base + 6] = old[u_base + 6];

    // F(0,3,6) -> D(0,3,6)
    stickers[d_base + 0] = old[f_base + 0];
    stickers[d_base + 3] = old[f_base + 3];
    stickers[d_base + 6] = old[f_base + 6];

    // D(0,3,6) -> B(8,5,2)
    stickers[b_base + 8] = old[d_base + 0];
    stickers[b_base + 5] = old[d_base + 3];
    stickers[b_base + 2] = old[d_base + 6];

    // B(8,5,2) -> U(6,3,0)
    stickers[u_base + 6] = old[b_base + 8];
    stickers[u_base + 3] = old[b_base + 5];
    stickers[u_base + 0] = old[b_base + 2];
}

fn apply_r_cw(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::R);

    let old = *stickers;
    let u_base = (Face::U as usize) * 9;
    let b_base = (Face::B as usize) * 9;
    let d_base = (Face::D as usize) * 9;
    let f_base = (Face::F as usize) * 9;

    // U(8,5,2) -> B(0,3,6)
    stickers[b_base + 0] = old[u_base + 8];
    stickers[b_base + 3] = old[u_base + 5];
    stickers[b_base + 6] = old[u_base + 2];

    // B(0,3,6) -> D(8,5,2)
    stickers[d_base + 8] = old[b_base + 0];
    stickers[d_base + 5] = old[b_base + 3];
    stickers[d_base + 2] = old[b_base + 6];

    // D(8,5,2) -> F(8,5,2)
    stickers[f_base + 8] = old[d_base + 8];
    stickers[f_base + 5] = old[d_base + 5];
    stickers[f_base + 2] = old[d_base + 2];

    // F(8,5,2) -> U(8,5,2)
    stickers[u_base + 8] = old[f_base + 8];
    stickers[u_base + 5] = old[f_base + 5];
    stickers[u_base + 2] = old[f_base + 2];
}

fn copy_face(src: &[Color; 54], src_face: Face, dst: &mut [Color; 54], dst_face: Face) {
    let s = (src_face as usize) * 9;
    let d = (dst_face as usize) * 9;
    dst[d..d + 9].copy_from_slice(&src[s..s + 9]);
}

fn apply_y_rot(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::U);
    rotate_face_ccw(stickers, Face::D);

    let old = *stickers;
    copy_face(&old, Face::F, stickers, Face::L);
    copy_face(&old, Face::L, stickers, Face::B);
    copy_face(&old, Face::B, stickers, Face::R);
    copy_face(&old, Face::R, stickers, Face::F);
}

fn apply_z_rot(stickers: &mut [Color; 54]) {
    rotate_face_cw(stickers, Face::F);
    rotate_face_ccw(stickers, Face::B);

    let old = *stickers;
    // U -> R, R -> D, D -> L, L -> U, each rotating cw
    let mut temp_u = [Color::White; 9];
    let mut temp_r = [Color::White; 9];
    let mut temp_d = [Color::White; 9];
    let mut temp_l = [Color::White; 9];

    let u_b = (Face::U as usize) * 9;
    let r_b = (Face::R as usize) * 9;
    let d_b = (Face::D as usize) * 9;
    let l_b = (Face::L as usize) * 9;

    temp_u.copy_from_slice(&old[u_b..u_b + 9]);
    temp_r.copy_from_slice(&old[r_b..r_b + 9]);
    temp_d.copy_from_slice(&old[d_b..d_b + 9]);
    temp_l.copy_from_slice(&old[l_b..l_b + 9]);

    fn rot90(a: &[Color; 9]) -> [Color; 9] {
        [a[6], a[3], a[0], a[7], a[4], a[1], a[8], a[5], a[2]]
    }

    stickers[r_b..r_b + 9].copy_from_slice(&rot90(&temp_u));
    stickers[d_b..d_b + 9].copy_from_slice(&rot90(&temp_r));
    stickers[l_b..l_b + 9].copy_from_slice(&rot90(&temp_d));
    stickers[u_b..u_b + 9].copy_from_slice(&rot90(&temp_l));
}
