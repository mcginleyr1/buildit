//! Stack types for Terraform/Infrastructure-as-Code management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stack status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StackStatus {
    Pending,
    Initializing,
    Ready,
    Error,
}

impl std::fmt::Display for StackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackStatus::Pending => write!(f, "pending"),
            StackStatus::Initializing => write!(f, "initializing"),
            StackStatus::Ready => write!(f, "ready"),
            StackStatus::Error => write!(f, "error"),
        }
    }
}

/// Stack run type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StackRunType {
    Plan,
    Apply,
    Destroy,
    Refresh,
}

impl std::fmt::Display for StackRunType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackRunType::Plan => write!(f, "plan"),
            StackRunType::Apply => write!(f, "apply"),
            StackRunType::Destroy => write!(f, "destroy"),
            StackRunType::Refresh => write!(f, "refresh"),
        }
    }
}

/// Stack run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StackRunStatus {
    Pending,
    Running,
    NeedsApproval,
    Approved,
    Applying,
    Succeeded,
    Failed,
    Cancelled,
}

impl std::fmt::Display for StackRunStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackRunStatus::Pending => write!(f, "pending"),
            StackRunStatus::Running => write!(f, "running"),
            StackRunStatus::NeedsApproval => write!(f, "needs_approval"),
            StackRunStatus::Approved => write!(f, "approved"),
            StackRunStatus::Applying => write!(f, "applying"),
            StackRunStatus::Succeeded => write!(f, "succeeded"),
            StackRunStatus::Failed => write!(f, "failed"),
            StackRunStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Trigger type for stack runs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StackTriggerType {
    Manual,
    Webhook,
    Drift,
    Scheduled,
}

impl std::fmt::Display for StackTriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StackTriggerType::Manual => write!(f, "manual"),
            StackTriggerType::Webhook => write!(f, "webhook"),
            StackTriggerType::Drift => write!(f, "drift"),
            StackTriggerType::Scheduled => write!(f, "scheduled"),
        }
    }
}

/// A Terraform stack (workspace)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub repository_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub terraform_version: String,
    pub auto_apply: bool,
    pub working_directory: Option<String>,
    pub var_file: Option<String>,
    pub backend_config: serde_json::Value,
    pub environment_variables: serde_json::Value,
    pub status: StackStatus,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Stack variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackVariable {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub key: String,
    pub value: Option<String>,
    pub is_sensitive: bool,
    pub is_hcl: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A Terraform run (plan/apply)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackRun {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub run_type: StackRunType,
    pub status: StackRunStatus,
    pub triggered_by: Option<Uuid>,
    pub trigger_type: StackTriggerType,
    pub commit_sha: Option<String>,
    pub plan_output: Option<String>,
    pub plan_json: Option<serde_json::Value>,
    pub apply_output: Option<String>,
    pub resources_to_add: i32,
    pub resources_to_change: i32,
    pub resources_to_destroy: i32,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Terraform state stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackState {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub state_json: serde_json::Value,
    pub serial: i32,
    pub lineage: Option<String>,
    pub lock_id: Option<String>,
    pub locked_by: Option<Uuid>,
    pub locked_at: Option<DateTime<Utc>>,
    pub lock_info: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

/// Parsed Terraform plan summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanSummary {
    pub to_add: Vec<ResourceChange>,
    pub to_change: Vec<ResourceChange>,
    pub to_destroy: Vec<ResourceChange>,
}

/// A resource change in a Terraform plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceChange {
    pub address: String,
    pub resource_type: String,
    pub name: String,
    pub action: String,
    pub before: Option<serde_json::Value>,
    pub after: Option<serde_json::Value>,
}

/// Request to create a stack
#[derive(Debug, Clone, Deserialize)]
pub struct CreateStackRequest {
    pub name: String,
    pub description: Option<String>,
    pub repository_id: Option<Uuid>,
    pub path: Option<String>,
    pub terraform_version: Option<String>,
    pub auto_apply: Option<bool>,
    pub variables: Option<Vec<CreateStackVariableRequest>>,
}

/// Request to create a stack variable
#[derive(Debug, Clone, Deserialize)]
pub struct CreateStackVariableRequest {
    pub key: String,
    pub value: Option<String>,
    pub is_sensitive: bool,
    pub is_hcl: bool,
    pub description: Option<String>,
}

/// Request to trigger a stack run
#[derive(Debug, Clone, Deserialize)]
pub struct TriggerStackRunRequest {
    pub run_type: StackRunType,
    pub commit_sha: Option<String>,
}
