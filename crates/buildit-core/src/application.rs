//! Application types for GitOps deployment.
//!
//! An Application represents a set of Kubernetes manifests that should be
//! deployed to a target environment. Similar to ArgoCD Applications.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Sync policy for an application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncPolicy {
    /// Manual sync required
    Manual,
    /// Auto-sync when git changes detected
    Auto,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        SyncPolicy::Manual
    }
}

impl std::fmt::Display for SyncPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncPolicy::Manual => write!(f, "manual"),
            SyncPolicy::Auto => write!(f, "auto"),
        }
    }
}

/// Application sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    /// Application is synced with git
    Synced,
    /// Application is out of sync with git
    OutOfSync,
    /// Sync is in progress
    Syncing,
    /// Unknown status (not yet checked)
    Unknown,
}

impl Default for SyncStatus {
    fn default() -> Self {
        SyncStatus::Unknown
    }
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::OutOfSync => write!(f, "out_of_sync"),
            SyncStatus::Syncing => write!(f, "syncing"),
            SyncStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Application health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All resources are healthy
    Healthy,
    /// Some resources are progressing
    Progressing,
    /// Some resources are degraded
    Degraded,
    /// Some resources are suspended
    Suspended,
    /// Some resources are missing
    Missing,
    /// Unknown health status
    Unknown,
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Unknown
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Progressing => write!(f, "progressing"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Suspended => write!(f, "suspended"),
            HealthStatus::Missing => write!(f, "missing"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// A GitOps Application that deploys Kubernetes manifests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub repository_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    /// Path to Kubernetes manifests in repository
    pub path: String,
    /// Target namespace for deployment
    pub target_namespace: String,
    /// Target cluster (uses environment's target if not specified)
    pub target_cluster: Option<String>,
    /// Sync policy (manual or auto)
    pub sync_policy: SyncPolicy,
    /// Whether to prune resources not in git
    pub prune: bool,
    /// Whether to auto-heal (revert manual changes)
    pub self_heal: bool,
    /// Current sync status
    pub sync_status: SyncStatus,
    /// Current health status
    pub health_status: HealthStatus,
    /// Current synced git revision
    pub synced_revision: Option<String>,
    /// Last sync timestamp
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A sync operation for an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSync {
    pub id: Uuid,
    pub application_id: Uuid,
    /// Git revision being synced
    pub revision: String,
    /// Status of the sync
    pub status: ApplicationSyncStatus,
    /// Who triggered the sync
    pub triggered_by: Option<Uuid>,
    /// Trigger type (manual, webhook, auto)
    pub trigger_type: SyncTriggerType,
    /// Resources created/updated/deleted counts
    pub resources_created: i32,
    pub resources_updated: i32,
    pub resources_deleted: i32,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,
    /// Finished timestamp
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Sync operation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationSyncStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

impl std::fmt::Display for ApplicationSyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplicationSyncStatus::Pending => write!(f, "pending"),
            ApplicationSyncStatus::Running => write!(f, "running"),
            ApplicationSyncStatus::Succeeded => write!(f, "succeeded"),
            ApplicationSyncStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Trigger type for sync operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncTriggerType {
    Manual,
    Webhook,
    Auto,
    Scheduled,
}

impl std::fmt::Display for SyncTriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncTriggerType::Manual => write!(f, "manual"),
            SyncTriggerType::Webhook => write!(f, "webhook"),
            SyncTriggerType::Auto => write!(f, "auto"),
            SyncTriggerType::Scheduled => write!(f, "scheduled"),
        }
    }
}

/// A Kubernetes resource managed by an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationResource {
    pub id: Uuid,
    pub application_id: Uuid,
    /// Kubernetes API group (e.g., "apps", "")
    pub api_group: String,
    /// Kubernetes API version (e.g., "v1", "apps/v1")
    pub api_version: String,
    /// Resource kind (e.g., "Deployment", "Service")
    pub kind: String,
    /// Resource name
    pub name: String,
    /// Resource namespace
    pub namespace: String,
    /// Current status
    pub status: ResourceStatus,
    /// Health status
    pub health_status: HealthStatus,
    /// Whether resource is out of sync
    pub out_of_sync: bool,
    /// Desired state (from git)
    pub desired_state: Option<serde_json::Value>,
    /// Live state (from cluster)
    pub live_state: Option<serde_json::Value>,
    /// Diff between desired and live
    pub diff: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Resource status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceStatus {
    /// Resource exists and matches desired state
    Synced,
    /// Resource exists but differs from desired state
    OutOfSync,
    /// Resource doesn't exist in cluster
    Missing,
    /// Resource exists in cluster but not in git (orphaned)
    Orphaned,
    /// Unknown status
    Unknown,
}

impl std::fmt::Display for ResourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceStatus::Synced => write!(f, "synced"),
            ResourceStatus::OutOfSync => write!(f, "out_of_sync"),
            ResourceStatus::Missing => write!(f, "missing"),
            ResourceStatus::Orphaned => write!(f, "orphaned"),
            ResourceStatus::Unknown => write!(f, "unknown"),
        }
    }
}
