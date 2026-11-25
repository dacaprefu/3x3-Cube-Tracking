use thiserror::Error;

#[derive(Error, Debug)]
pub enum GyroError {
    #[error("Cuaternión inválido o no normalizado")]
    InvalidQuaternion,

    #[error("Error de orientación: {0}")]
    OrientationError(String),
}
