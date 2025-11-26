//! Application state.

use buildit_db::PgPipelineRepo;
use buildit_db::PgTenantRepo;
use buildit_executor::LocalDockerExecutor;
use buildit_scheduler::PipelineOrchestrator;
use sqlx::PgPool;
use std::sync::Arc;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tenant_repo: Arc<PgTenantRepo>,
    pub pipeline_repo: Arc<PgPipelineRepo>,
    pub orchestrator: Arc<PipelineOrchestrator>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let tenant_repo = Arc::new(PgTenantRepo::new(pool.clone()));
        let pipeline_repo = Arc::new(PgPipelineRepo::new(pool.clone()));

        // Create executor and orchestrator
        let executor = LocalDockerExecutor::new().expect("Failed to connect to Docker");
        let orchestrator = Arc::new(PipelineOrchestrator::new(Arc::new(executor)));

        Self {
            pool,
            tenant_repo,
            pipeline_repo,
            orchestrator,
        }
    }
}
