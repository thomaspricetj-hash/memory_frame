use thiserror::Error;

/// Unified error type for all memory-frame operations.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("slice not found: {0}")]
    SliceNotFound(String),

    #[error("cell not found: {0}")]
    CellNotFound(String),

    #[error("model adapter error: {0}")]
    AdapterError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("deserialization error: {0}")]
    DeserializationError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("unexpected error: {0}")]
    Unexpected(String),
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Unexpected(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::SerializationError(err.to_string())
    }
}

impl From<bincode::Error> for ApiError {
    fn from(err: bincode::Error) -> Self {
        ApiError::SerializationError(err.to_string())
    }
}

impl From<chrono::ParseError> for ApiError {
    fn from(err: chrono::ParseError) -> Self {
        ApiError::ParseError(err.to_string())
    }
}

impl From<String> for ApiError {
    fn from(err: String) -> Self {
        ApiError::Unexpected(err)
    }
}

impl From<&str> for ApiError {
    fn from(err: &str) -> Self {
        ApiError::Unexpected(err.to_string())
    }
}






