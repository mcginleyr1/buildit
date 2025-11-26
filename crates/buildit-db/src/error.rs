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

    #[error("postgres error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[error("pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
}

pub type DbResult<T> = std::result::Result<T, DbError>;
