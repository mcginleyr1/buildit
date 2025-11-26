//! Pipeline and stage definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ResourceId;
use crate::deployer::DeploymentSpec;
use crate::executor::JobSpec;

/// A CI/CD pipeline definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    /// Unique identifier.
    pub id: ResourceId,
    /// Pipeline name (e.g., "my-service").
    pub name: String,
    /// Tenant this pipeline belongs to.
    pub tenant_id: ResourceId,
    /// Repository URL.
    pub repository: String,
    /// Triggers that can start this pipeline.
    pub triggers: Vec<Trigger>,
    /// Pipeline stages.
    pub stages: Vec<Stage>,
    /// Global environment variables.
    pub env: HashMap<String, String>,
    /// Cache configurations.
    pub caches: Vec<CacheConfig>,
}

/// What triggers a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    /// Triggered on push to branches.
    Push {
        branches: Vec<String>,
        paths: Option<Vec<String>>,
    },
    /// Triggered on pull request.
    PullRequest { branches: Option<Vec<String>> },
    /// Triggered on tag creation.
    Tag { pattern: Option<String> },
    /// Scheduled trigger (cron).
    Schedule { cron: String },
    /// Manual trigger only.
    Manual,
    /// Triggered via API/webhook.
    Webhook { secret: String },
}

/// A stage in a pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// Stage name.
    pub name: String,
    /// Dependencies (other stage names).
    pub needs: Vec<String>,
    /// Conditional execution.
    pub when: Option<StageCondition>,
    /// Whether manual approval is required.
    pub manual: bool,
    /// What this stage does.
    pub action: StageAction,
    /// Stage-specific environment variables.
    pub env: HashMap<String, String>,
}

/// Condition for stage execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageCondition {
    /// Expression to evaluate (e.g., "{branch} == 'main'").
    pub expression: String,
}

/// What a stage does.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageAction {
    /// Run commands in a container.
    Run {
        image: String,
        commands: Vec<String>,
        artifacts: Vec<String>,
    },
    /// Build and push a container image.
    ImageBuild {
        dockerfile: String,
        context: String,
        tags: Vec<String>,
        push: bool,
    },
    /// Deploy to a target.
    Deploy(Box<DeploymentSpec>),
    /// Run stages in parallel.
    Parallel { stages: Vec<Stage> },
    /// Matrix build (multiple configurations).
    Matrix {
        variables: HashMap<String, Vec<String>>,
        stage: Box<Stage>,
    },
}

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache name.
    pub name: String,
    /// Paths to cache.
    pub paths: Vec<String>,
    /// Cache key (supports variable interpolation).
    pub key: String,
    /// Fallback keys if exact match not found.
    pub restore_keys: Vec<String>,
}

/// A pipeline run instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRun {
    /// Unique identifier.
    pub id: ResourceId,
    /// Pipeline definition ID.
    pub pipeline_id: ResourceId,
    /// Run number (incrementing).
    pub number: u64,
    /// What triggered this run.
    pub trigger: TriggerInfo,
    /// Git information.
    pub git: GitInfo,
    /// Current status.
    pub status: PipelineStatus,
    /// Stage results.
    pub stages: Vec<StageResult>,
    /// When the run was created.
    pub created_at: DateTime<Utc>,
    /// When the run started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the run finished.
    pub finished_at: Option<DateTime<Utc>>,
}

/// Information about what triggered a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerInfo {
    pub kind: TriggerKind,
    pub actor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerKind {
    Push,
    PullRequest { number: u64 },
    Tag { name: String },
    Schedule,
    Manual,
    Webhook,
    Retry { original_run_id: ResourceId },
}

/// Git information for a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfo {
    pub sha: String,
    pub short_sha: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub message: String,
    pub author: String,
}

/// Overall pipeline status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    /// Waiting to start.
    Queued,
    /// Currently running.
    Running,
    /// Waiting for manual approval.
    WaitingApproval { stage: String },
    /// Completed successfully.
    Succeeded,
    /// Failed.
    Failed { stage: String },
    /// Cancelled.
    Cancelled,
}

impl PipelineStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            PipelineStatus::Succeeded | PipelineStatus::Failed { .. } | PipelineStatus::Cancelled
        )
    }
}

/// Result of a stage execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    /// Stage name.
    pub name: String,
    /// Status.
    pub status: StageStatus,
    /// Job handle if this was a run/build stage.
    pub job_id: Option<ResourceId>,
    /// Deployment handle if this was a deploy stage.
    pub deployment_id: Option<ResourceId>,
    /// When the stage started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the stage finished.
    pub finished_at: Option<DateTime<Utc>>,
}

/// Status of a stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    /// Waiting for dependencies.
    Pending,
    /// Ready to run but queued.
    Queued,
    /// Currently executing.
    Running,
    /// Waiting for manual approval.
    WaitingApproval,
    /// Completed successfully.
    Succeeded,
    /// Failed.
    Failed { message: String },
    /// Skipped (condition not met or dependency failed).
    Skipped { reason: String },
    /// Cancelled.
    Cancelled,
}
