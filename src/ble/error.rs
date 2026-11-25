use thiserror::Error;

#[derive(Error, Debug)]
pub enum BleError {
    #[error("No se encontró adaptador Bluetooth disponible")]
    NoAdapterAvailable,

    #[error("Dispositivo GAN no encontrado tras escanear")]
    DeviceNotFound,

    #[error("Fallo al conectar con el dispositivo BLE: {0}")]
    ConnectionFailed(String),

    #[error("Fallo al obtener servicios o características BLE: {0}")]
    ServiceDiscoveryFailed(String),

    #[error("No se pudo extraer la dirección MAC del cubo desde los datos del anuncio o adaptador")]
    MacExtractionFailed,

    #[error("Error de desencriptación AES-128: {0}")]
    DecryptionFailed(String),

    #[error("Longitud de paquete inválida: esperados al menos {expected} bytes, recibidos {actual}")]
    InvalidPacketLength { expected: usize, actual: usize },

    #[error("Cabecera de evento desconocida: 0x{0:02X}")]
    UnknownEventHeader(u8),

    #[error("Error en btleplug: {0}")]
    Btleplug(String),

    #[error("Error de decodificación de datos: {0}")]
    DecodeError(String),
}

impl From<btleplug::Error> for BleError {
    fn from(err: btleplug::Error) -> Self {
        BleError::Btleplug(err.to_string())
    }
}
