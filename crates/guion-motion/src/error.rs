use std::fmt;

#[derive(Debug)]
pub enum MotionError {
    Io(std::io::Error),
    Wasm(String),
    MissingExport(String),
    MemoryBounds,
    ParamCount { expected: usize, got: usize },
    UnknownField(String),
}

impl fmt::Display for MotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MotionError::Io(e) => write!(f, "{e}"),
            MotionError::Wasm(e) => write!(f, "wasm: {e}"),
            MotionError::MissingExport(name) => write!(f, "export faltante: {name}"),
            MotionError::MemoryBounds => write!(f, "fuera de memoria wasm"),
            MotionError::ParamCount { expected, got } => {
                write!(f, "esperaba {expected} params, recibí {got}")
            }
            MotionError::UnknownField(field) => write!(f, "campo desconocido: {field}"),
        }
    }
}

impl std::error::Error for MotionError {}

impl From<std::io::Error> for MotionError {
    fn from(e: std::io::Error) -> Self {
        MotionError::Io(e)
    }
}
