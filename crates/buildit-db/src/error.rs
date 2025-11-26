//! Database error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("duplicate: {0}")]
    Duplicate(String),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

pub type DbResult<T> = std::result::Result<T, DbError>;
