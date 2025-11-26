//! Database layer for BuildIt CI/CD.
//!
//! Provides repository traits and implementations using Clorinde-generated queries.

pub mod error;
pub mod repo;

pub use error::{DbError, DbResult};
pub use repo::*;

// Re-export generated query types
pub use buildit_db_queries::queries::{jobs, pipelines, tenants};

use deadpool_postgres::{Config, Pool, Runtime};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio_postgres::NoTls;

/// Create a new SQLx database connection pool (for migrations).
pub async fn create_pool(database_url: &str) -> DbResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Create a deadpool-postgres pool for Clorinde queries.
pub fn create_deadpool(database_url: &str) -> DbResult<Pool> {
    let mut cfg = Config::new();
    cfg.url = Some(database_url.to_string());
    let pool = cfg
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .map_err(|e| DbError::Connection(e.to_string()))?;
    Ok(pool)
}

/// Run database migrations.
pub async fn run_migrations(pool: &PgPool) -> DbResult<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
