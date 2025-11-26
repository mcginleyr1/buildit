//! Application state.

use buildit_db::PgDeploymentRepo;
use buildit_db::PgOrganizationRepo;
use buildit_db::PgPipelineRepo;
use buildit_db::PgTenantRepo;
use buildit_executor::LocalDockerExecutor;
use buildit_scheduler::PipelineOrchestrator;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tenant_repo: Arc<PgTenantRepo>,
    pub pipeline_repo: Arc<PgPipelineRepo>,
    pub deployment_repo: Arc<PgDeploymentRepo>,
    pub organization_repo: Arc<PgOrganizationRepo>,
    pub orchestrator: Option<Arc<PipelineOrchestrator>>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let tenant_repo = Arc::new(PgTenantRepo::new(pool.clone()));
        let pipeline_repo = Arc::new(PgPipelineRepo::new(pool.clone()));
        let deployment_repo = Arc::new(PgDeploymentRepo::new(pool.clone()));
        let organization_repo = Arc::new(PgOrganizationRepo::new(pool.clone()));

        // Try to create executor and orchestrator - may fail in K8s without Docker socket
        let orchestrator = match LocalDockerExecutor::new() {
            Ok(executor) => {
                info!("Docker executor initialized successfully");
                Some(Arc::new(PipelineOrchestrator::new(Arc::new(executor))))
            }
            Err(e) => {
                warn!(
                    "Docker executor unavailable: {}. Pipeline execution disabled.",
                    e
                );
                None
            }
        };

        Self {
            pool,
            tenant_repo,
            pipeline_repo,
            deployment_repo,
            organization_repo,
            orchestrator,
        }
    }
}
