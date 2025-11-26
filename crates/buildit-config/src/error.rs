//! Configuration parsing errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("KDL parse error: {0}")]
    Parse(#[from] kdl::KdlError),

    #[error("missing required field: {0}")]
    MissingField(String),

    #[error("invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },

    #[error("duplicate definition: {0}")]
    Duplicate(String),

    #[error("invalid reference: {0}")]
    InvalidReference(String),

    #[error("cycle detected in dependencies: {0}")]
    CycleDetected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type ConfigResult<T> = std::result::Result<T, ConfigError>;
