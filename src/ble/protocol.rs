use crate::ble::error::BleError;
use crate::cube::color::{Color, Face};
use crate::cube::moves::Move;
use crate::cube::state::Cube;
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

pub const HEADER_MOVE: u8 = 0x01;
pub const HEADER_FACELETS: u8 = 0x02;
pub const HEADER_GYRO: u8 = 0x03;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveDirection {
    Clockwise = 0,
    CounterClockwise = 1,
    Double = 2,
}

impl MoveDirection {
    pub fn from_u8(val: u8) -> Result<Self, BleError> {
        match val {
            0 => Ok(MoveDirection::Clockwise),
            1 => Ok(MoveDirection::CounterClockwise),
            2 => Ok(MoveDirection::Double),
            _ => Err(BleError::DecodeError(format!(
                "Dirección de movimiento inválida: 0x{:02X}",
                val
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GanMoveEvent {
    pub face: Face,
    pub direction: MoveDirection,
    pub hardware_timestamp_ms: u64,
    pub sequence_number: u16,
}

impl GanMoveEvent {
    /// Convierte el evento de giro del hardware a la representación de movimiento `src/cube::Move`.
    pub fn to_cube_move(&self) -> Move {
        match (self.face, self.direction) {
            (Face::U, MoveDirection::Clockwise) => Move::U,
            (Face::U, MoveDirection::CounterClockwise) => Move::UPrime,
            (Face::U, MoveDirection::Double) => Move::U2,

            (Face::D, MoveDirection::Clockwise) => Move::D,
            (Face::D, MoveDirection::CounterClockwise) => Move::DPrime,
            (Face::D, MoveDirection::Double) => Move::D2,

            (Face::F, MoveDirection::Clockwise) => Move::F,
            (Face::F, MoveDirection::CounterClockwise) => Move::FPrime,
            (Face::F, MoveDirection::Double) => Move::F2,

            (Face::B, MoveDirection::Clockwise) => Move::B,
            (Face::B, MoveDirection::CounterClockwise) => Move::BPrime,
            (Face::B, MoveDirection::Double) => Move::B2,

            (Face::L, MoveDirection::Clockwise) => Move::L,
            (Face::L, MoveDirection::CounterClockwise) => Move::LPrime,
            (Face::L, MoveDirection::Double) => Move::L2,

            (Face::R, MoveDirection::Clockwise) => Move::R,
            (Face::R, MoveDirection::CounterClockwise) => Move::RPrime,
            (Face::R, MoveDirection::Double) => Move::R2,
        }
    }
}

mod serde_54_colors {
    use super::*;
    use serde::de::SeqAccess;
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(stickers: &[Color; 54], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(54))?;
        for c in stickers {
            seq.serialize_element(c)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[Color; 54], D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ColorVisitor;
        impl<'de> serde::de::Visitor<'de> for ColorVisitor {
            type Value = [Color; 54];
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("array of 54 Colors")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut arr = [Color::White; 54];
                for i in 0..54 {
                    arr[i] = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(arr)
            }
        }
        deserializer.deserialize_seq(ColorVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GanFaceletsEvent {
    pub hardware_timestamp_ms: u64,
    #[serde(with = "serde_54_colors")]
    pub facelets: [Color; 54],
}

impl GanFaceletsEvent {
    /// Comprueba si el estado de stickers recibido coincide exactamente con el cubo interno
    /// para prevenir desincronizaciones.
    pub fn verify_matches_cube(&self, cube: &Cube) -> bool {
        let current_stickers = cube.to_stickers();
        self.facelets == current_stickers
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GanGyroEvent {
    pub hardware_timestamp_ms: u64,
    pub quaternion: Quat,
    pub angular_velocity: Vec3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GanEvent {
    Move(GanMoveEvent),
    Facelets(GanFaceletsEvent),
    Gyro(GanGyroEvent),
}

impl GanEvent {
    pub fn hardware_timestamp_ms(&self) -> u64 {
        match self {
            GanEvent::Move(m) => m.hardware_timestamp_ms,
            GanEvent::Facelets(f) => f.hardware_timestamp_ms,
            GanEvent::Gyro(g) => g.hardware_timestamp_ms,
        }
    }
}

fn face_from_u8(val: u8) -> Result<Face, BleError> {
    match val {
        0 => Ok(Face::U),
        1 => Ok(Face::D),
        2 => Ok(Face::F),
        3 => Ok(Face::B),
        4 => Ok(Face::L),
        5 => Ok(Face::R),
        _ => Err(BleError::DecodeError(format!("Cara inválida: 0x{:02X}", val))),
    }
}

fn color_from_u8(val: u8) -> Result<Color, BleError> {
    match val {
        0 => Ok(Color::White),
        1 => Ok(Color::Yellow),
        2 => Ok(Color::Green),
        3 => Ok(Color::Blue),
        4 => Ok(Color::Orange),
        5 => Ok(Color::Red),
        _ => Err(BleError::DecodeError(format!("Color inválido: 0x{:02X}", val))),
    }
}

fn color_to_u8(c: Color) -> u8 {
    match c {
        Color::White => 0,
        Color::Yellow => 1,
        Color::Green => 2,
        Color::Blue => 3,
        Color::Orange => 4,
        Color::Red => 5,
    }
}

/// Decodifica un payload desencriptado según su cabecera de evento.
pub fn decode_packet(data: &[u8]) -> Result<GanEvent, BleError> {
    if data.is_empty() {
        return Err(BleError::InvalidPacketLength {
            expected: 1,
            actual: 0,
        });
    }

    let header = data[0];
    match header {
        HEADER_MOVE => {
            // Cabecera (1) + Cara (1) + Dirección (1) + Timestamp (4) + Seq (2) = 9 bytes
            if data.len() < 9 {
                return Err(BleError::InvalidPacketLength {
                    expected: 9,
                    actual: data.len(),
                });
            }

            let face = face_from_u8(data[1])?;
            let direction = MoveDirection::from_u8(data[2])?;
            let ts_bytes: [u8; 4] = [data[3], data[4], data[5], data[6]];
            let hardware_timestamp_ms = u32::from_be_bytes(ts_bytes) as u64;
            let seq_bytes: [u8; 2] = [data[7], data[8]];
            let sequence_number = u16::from_be_bytes(seq_bytes);

            Ok(GanEvent::Move(GanMoveEvent {
                face,
                direction,
                hardware_timestamp_ms,
                sequence_number,
            }))
        }
        HEADER_FACELETS => {
            // Cabecera (1) + Timestamp (4) + 54 stickers (54) = 59 bytes
            if data.len() < 59 {
                return Err(BleError::InvalidPacketLength {
                    expected: 59,
                    actual: data.len(),
                });
            }

            let ts_bytes: [u8; 4] = [data[1], data[2], data[3], data[4]];
            let hardware_timestamp_ms = u32::from_be_bytes(ts_bytes) as u64;

            let mut facelets = [Color::White; 54];
            for i in 0..54 {
                facelets[i] = color_from_u8(data[5 + i])?;
            }

            Ok(GanEvent::Facelets(GanFaceletsEvent {
                hardware_timestamp_ms,
                facelets,
            }))
        }
        HEADER_GYRO => {
            // Cabecera (1) + Timestamp (4) + Quat (8) + AngVel (6) = 19 bytes
            if data.len() < 19 {
                return Err(BleError::InvalidPacketLength {
                    expected: 19,
                    actual: data.len(),
                });
            }

            let ts_bytes: [u8; 4] = [data[1], data[2], data[3], data[4]];
            let hardware_timestamp_ms = u32::from_be_bytes(ts_bytes) as u64;

            let scale = 16384.0f32;
            let w = i16::from_be_bytes([data[5], data[6]]) as f32 / scale;
            let x = i16::from_be_bytes([data[7], data[8]]) as f32 / scale;
            let y = i16::from_be_bytes([data[9], data[10]]) as f32 / scale;
            let z = i16::from_be_bytes([data[11], data[12]]) as f32 / scale;

            let quaternion = Quat::from_xyzw(x, y, z, w).normalize();

            let vel_scale = 100.0f32;
            let vx = i16::from_be_bytes([data[13], data[14]]) as f32 / vel_scale;
            let vy = i16::from_be_bytes([data[15], data[16]]) as f32 / vel_scale;
            let vz = i16::from_be_bytes([data[17], data[18]]) as f32 / vel_scale;
            let angular_velocity = Vec3::new(vx, vy, vz);

            Ok(GanEvent::Gyro(GanGyroEvent {
                hardware_timestamp_ms,
                quaternion,
                angular_velocity,
            }))
        }
        _ => Err(BleError::UnknownEventHeader(header)),
    }
}

/// Codifica un evento para generar paquetes sin encriptar (útil para tests y simulación de hardware).
pub fn encode_packet(event: &GanEvent) -> Vec<u8> {
    match event {
        GanEvent::Move(m) => {
            let mut buf = Vec::with_capacity(9);
            buf.push(HEADER_MOVE);
            buf.push(m.face as u8);
            buf.push(m.direction as u8);
            buf.extend_from_slice(&(m.hardware_timestamp_ms as u32).to_be_bytes());
            buf.extend_from_slice(&m.sequence_number.to_be_bytes());
            buf
        }
        GanEvent::Facelets(f) => {
            let mut buf = Vec::with_capacity(59);
            buf.push(HEADER_FACELETS);
            buf.extend_from_slice(&(f.hardware_timestamp_ms as u32).to_be_bytes());
            for c in f.facelets {
                buf.push(color_to_u8(c));
            }
            buf
        }
        GanEvent::Gyro(g) => {
            let mut buf = Vec::with_capacity(19);
            buf.push(HEADER_GYRO);
            buf.extend_from_slice(&(g.hardware_timestamp_ms as u32).to_be_bytes());

            let scale = 16384.0f32;
            let w = (g.quaternion.w * scale).round() as i16;
            let x = (g.quaternion.x * scale).round() as i16;
            let y = (g.quaternion.y * scale).round() as i16;
            let z = (g.quaternion.z * scale).round() as i16;

            buf.extend_from_slice(&w.to_be_bytes());
            buf.extend_from_slice(&x.to_be_bytes());
            buf.extend_from_slice(&y.to_be_bytes());
            buf.extend_from_slice(&z.to_be_bytes());

            let vel_scale = 100.0f32;
            let vx = (g.angular_velocity.x * vel_scale).round() as i16;
            let vy = (g.angular_velocity.y * vel_scale).round() as i16;
            let vz = (g.angular_velocity.z * vel_scale).round() as i16;

            buf.extend_from_slice(&vx.to_be_bytes());
            buf.extend_from_slice(&vy.to_be_bytes());
            buf.extend_from_slice(&vz.to_be_bytes());
            buf
        }
    }
}
