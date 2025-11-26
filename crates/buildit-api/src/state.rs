//! Application state.

use buildit_db::PgPipelineRepo;
use buildit_db::PgTenantRepo;
use sqlx::PgPool;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tenant_repo: Arc<PgTenantRepo>,
    pub pipeline_repo: Arc<PgPipelineRepo>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let tenant_repo = Arc::new(PgTenantRepo::new(pool.clone()));
        let pipeline_repo = Arc::new(PgPipelineRepo::new(pool.clone()));

        Self {
            pool,
            tenant_repo,
            pipeline_repo,
        }
    }
}
