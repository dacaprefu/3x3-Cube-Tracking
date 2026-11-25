use thiserror::Error;

#[derive(Error, Debug)]
pub enum AnalyticsError {
    #[error("No hay suficientes movimientos para analizar el solve")]
    InsufficientMoves,

    #[error("El solve no está completo o no finalizó")]
    SolveIncomplete,

    #[error("Error de análisis: {0}")]
    AnalysisError(String),
}
