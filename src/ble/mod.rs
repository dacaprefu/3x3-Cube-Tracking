pub mod clock_sync;
pub mod crypto;
pub mod error;
pub mod protocol;
pub mod scanner;

#[cfg(test)]
mod tests;

pub use clock_sync::{AlignedTimestamp, ClockSync};
pub use crypto::{derive_key_iv, extract_mac_from_properties, GanCipher, GAN_ROOT_IV, GAN_ROOT_KEY};
pub use error::BleError;
pub use protocol::{
    decode_packet, encode_packet, GanEvent, GanFaceletsEvent, GanGyroEvent, GanMoveEvent,
    MoveDirection, HEADER_FACELETS, HEADER_GYRO, HEADER_MOVE,
};
pub use scanner::{
    scan_for_gan_cubes, DiscoveredGanCube, GanBleClient, GAN_DATA_SERVICE_UUID,
    GAN_NOTIFY_CHAR_UUID, GAN_WRITE_CHAR_UUID,
};
