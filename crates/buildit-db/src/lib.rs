//! Database layer for BuildIt CI/CD.
//!
//! Provides repository traits and PostgreSQL implementations.

pub mod error;
pub mod repo;

pub use error::{DbError, DbResult};
pub use repo::*;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// Create a new database connection pool.
pub async fn create_pool(database_url: &str) -> DbResult<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;
    Ok(pool)
}

/// Run database migrations.
pub async fn run_migrations(pool: &PgPool) -> DbResult<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}
