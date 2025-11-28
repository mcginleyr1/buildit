//! Application state.

use buildit_db::PgApplicationRepo;
use buildit_db::PgDeploymentRepo;
use buildit_db::PgLogRepo;
use buildit_db::PgOrganizationRepo;
use buildit_db::PgPipelineRepo;
use buildit_db::PgRepositoryRepo;
use buildit_db::PgStackRepo;
use buildit_db::PgTenantRepo;

use crate::ws::Broadcaster;
use buildit_executor::{KubernetesExecutor, LocalDockerExecutor};
use buildit_scheduler::PipelineOrchestrator;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::{info, warn};

/// Executor type to use for pipeline execution.
#[derive(Debug, Clone, Default)]
pub enum ExecutorType {
    /// Use Kubernetes Jobs (default when running in K8s)
    Kubernetes,
    /// Use local Docker containers
    #[default]
    Docker,
}

impl ExecutorType {
    /// Determine executor type from environment.
    /// Uses BUILDIT_EXECUTOR env var, or auto-detects based on environment.
    pub fn from_env() -> Self {
        // Check explicit configuration first
        if let Ok(executor) = std::env::var("BUILDIT_EXECUTOR") {
            match executor.to_lowercase().as_str() {
                "kubernetes" | "k8s" => return Self::Kubernetes,
                "docker" | "local" => return Self::Docker,
                other => {
                    warn!("Unknown executor type '{}', using auto-detection", other);
                }
            }
        }

        // Auto-detect: if we're running in K8s (KUBERNETES_SERVICE_HOST is set), use K8s executor
        if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
            info!("Detected Kubernetes environment, using Kubernetes executor");
            Self::Kubernetes
        } else {
            info!("Using Docker executor");
            Self::Docker
        }
    }
}

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub tenant_repo: Arc<PgTenantRepo>,
    pub pipeline_repo: Arc<PgPipelineRepo>,
    pub deployment_repo: Arc<PgDeploymentRepo>,
    pub organization_repo: Arc<PgOrganizationRepo>,
    pub repository_repo: Arc<PgRepositoryRepo>,
    pub stack_repo: Arc<PgStackRepo>,
    pub application_repo: Arc<PgApplicationRepo>,
    pub log_repo: Arc<PgLogRepo>,
    pub broadcaster: Arc<Broadcaster>,
    pub orchestrator: Option<Arc<PipelineOrchestrator>>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        let tenant_repo = Arc::new(PgTenantRepo::new(pool.clone()));
        let pipeline_repo = Arc::new(PgPipelineRepo::new(pool.clone()));
        let deployment_repo = Arc::new(PgDeploymentRepo::new(pool.clone()));
        let organization_repo = Arc::new(PgOrganizationRepo::new(pool.clone()));
        let repository_repo = Arc::new(PgRepositoryRepo::new(pool.clone()));
        let stack_repo = Arc::new(PgStackRepo::new(pool.clone()));
        let application_repo = Arc::new(PgApplicationRepo::new(pool.clone()));
        let log_repo = Arc::new(PgLogRepo::new(pool.clone()));
        let broadcaster = Arc::new(Broadcaster::new());

        // Orchestrator is initialized async via init_executor()
        let orchestrator = None;

        Self {
            pool,
            tenant_repo,
            pipeline_repo,
            deployment_repo,
            organization_repo,
            repository_repo,
            stack_repo,
            application_repo,
            log_repo,
            broadcaster,
            orchestrator,
        }
    }

    /// Initialize the executor asynchronously (required for Kubernetes executor).
    pub async fn init_executor(&mut self, executor_type: ExecutorType) {
        let namespace =
            std::env::var("BUILDIT_JOB_NAMESPACE").unwrap_or_else(|_| "buildit".to_string());

        match executor_type {
            ExecutorType::Kubernetes => match KubernetesExecutor::new(&namespace).await {
                Ok(executor) => {
                    info!(namespace = %namespace, "Kubernetes executor initialized");
                    self.orchestrator =
                        Some(Arc::new(PipelineOrchestrator::new(Arc::new(executor))));
                }
                Err(e) => {
                    warn!(
                        "Kubernetes executor unavailable: {}. Pipeline execution disabled.",
                        e
                    );
                }
            },
            ExecutorType::Docker => match LocalDockerExecutor::new() {
                Ok(executor) => {
                    info!("Docker executor initialized");
                    self.orchestrator =
                        Some(Arc::new(PipelineOrchestrator::new(Arc::new(executor))));
                }
                Err(e) => {
                    warn!(
                        "Docker executor unavailable: {}. Pipeline execution disabled.",
                        e
                    );
                }
            },
        }
    }
}
