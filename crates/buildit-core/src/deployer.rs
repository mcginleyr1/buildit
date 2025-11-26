//! Deployer trait and deployment types.
//!
//! Deployers manage application deployments to various targets (K8s, Fly.io, etc.)

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::executor::{LogLine, TerminalSession};
use crate::{ResourceId, Result};

/// Specification for a deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSpec {
    /// Unique identifier for this deployment.
    pub id: ResourceId,
    /// Service name being deployed.
    pub service: String,
    /// Target environment.
    pub environment: String,
    /// Container image to deploy.
    pub image: String,
    /// Number of replicas.
    pub replicas: u32,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Deployment strategy.
    pub strategy: DeploymentStrategy,
    /// Resource requirements.
    pub resources: DeploymentResources,
    /// Health check configuration.
    pub health_check: Option<HealthCheck>,
}

/// Deployment strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    /// Replace all instances at once.
    RollingUpdate {
        max_surge: u32,
        max_unavailable: u32,
    },
    /// Gradually shift traffic to new version.
    Canary { steps: Vec<CanaryStep> },
    /// Deploy new version alongside old, then switch.
    BlueGreen,
    /// Recreate all instances (downtime).
    Recreate,
}

impl Default for DeploymentStrategy {
    fn default() -> Self {
        Self::RollingUpdate {
            max_surge: 1,
            max_unavailable: 0,
        }
    }
}

/// A step in a canary deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryStep {
    /// Percentage of traffic to send to new version.
    pub traffic_percent: u8,
    /// How long to wait before proceeding.
    pub duration: Option<std::time::Duration>,
    /// Whether manual approval is required.
    pub manual_approval: bool,
}

/// Resource configuration for a deployment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentResources {
    pub cpu_limit: Option<String>,
    pub memory_limit: Option<String>,
    pub cpu_request: Option<String>,
    pub memory_request: Option<String>,
}

/// Health check configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub path: String,
    pub port: u16,
    pub interval_seconds: u32,
    pub timeout_seconds: u32,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

/// Handle to an active deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentHandle {
    pub id: ResourceId,
    pub deployer_id: String,
    pub deployer_name: String,
}

/// Current state of a deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentState {
    pub status: DeploymentStatus,
    pub replicas: ReplicaStatus,
    pub current_image: String,
    pub traffic_distribution: Option<TrafficDistribution>,
    pub last_updated: DateTime<Utc>,
}

/// Status of a deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment is starting.
    Pending,
    /// Deployment is in progress.
    InProgress { progress_percent: u8 },
    /// Deployment completed successfully.
    Healthy,
    /// Deployment is degraded but functional.
    Degraded { message: String },
    /// Deployment failed.
    Failed { message: String },
    /// Deployment is paused (canary).
    Paused { reason: String },
    /// Deployment is being rolled back.
    RollingBack,
}

/// Replica status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicaStatus {
    pub desired: u32,
    pub ready: u32,
    pub available: u32,
    pub unavailable: u32,
}

/// Traffic distribution between versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficDistribution {
    pub stable_percent: u8,
    pub canary_percent: u8,
}

/// Deployment event for real-time updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEvent {
    pub timestamp: DateTime<Utc>,
    pub kind: DeploymentEventKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentEventKind {
    Started,
    ReplicaReady,
    ReplicaFailed,
    TrafficShifted,
    HealthCheckPassed,
    HealthCheckFailed,
    Paused,
    Resumed,
    Completed,
    Failed,
    RollbackStarted,
    RollbackCompleted,
}

/// Target for a rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackTarget {
    /// Roll back to the previous version.
    Previous,
    /// Roll back to a specific revision.
    Revision(u32),
    /// Roll back to a specific image.
    Image(String),
}

/// Options for log retrieval.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogOptions {
    pub since: Option<DateTime<Utc>>,
    pub tail_lines: Option<u32>,
    pub follow: bool,
    pub instance: Option<String>,
}

/// Warning from deployment validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
}

/// Trait for deployers.
#[async_trait]
pub trait Deployer: Send + Sync {
    /// Name of this deployer.
    fn name(&self) -> &'static str;

    /// Supported deployment strategies.
    fn supported_strategies(&self) -> Vec<DeploymentStrategy>;

    /// Validate a deployment spec before deploying.
    async fn validate(&self, spec: &DeploymentSpec) -> Result<Vec<ValidationWarning>>;

    /// Start a deployment.
    async fn deploy(&self, spec: DeploymentSpec) -> Result<DeploymentHandle>;

    /// Get current deployment state.
    async fn state(&self, handle: &DeploymentHandle) -> Result<DeploymentState>;

    /// Stream deployment events.
    async fn events(
        &self,
        handle: &DeploymentHandle,
    ) -> Result<BoxStream<'static, DeploymentEvent>>;

    /// Rollback a deployment.
    async fn rollback(
        &self,
        handle: &DeploymentHandle,
        target: RollbackTarget,
    ) -> Result<DeploymentHandle>;

    /// Scale a deployment.
    async fn scale(&self, handle: &DeploymentHandle, replicas: u32) -> Result<()>;

    /// Pause a deployment (canary).
    async fn pause(&self, handle: &DeploymentHandle) -> Result<()>;

    /// Resume a paused deployment.
    async fn resume(&self, handle: &DeploymentHandle) -> Result<()>;

    /// Destroy/delete a deployment.
    async fn destroy(&self, handle: &DeploymentHandle) -> Result<()>;

    /// Stream logs from a deployment.
    async fn logs(
        &self,
        handle: &DeploymentHandle,
        opts: LogOptions,
    ) -> Result<BoxStream<'static, LogLine>>;

    /// Open an interactive session to a deployment instance.
    async fn exec(
        &self,
        handle: &DeploymentHandle,
        instance: Option<String>,
        cmd: Vec<String>,
    ) -> Result<TerminalSession>;
}
