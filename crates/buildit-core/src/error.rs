//! Error types for BuildIt.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("deployment failed: {0}")]
    DeploymentFailed(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("cancelled")]
    Cancelled,

    #[error("internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, Error>;
