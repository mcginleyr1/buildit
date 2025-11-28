//! Executor trait and job types.
//!
//! Executors run CI jobs in isolated environments (containers, pods, etc.)

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::{ResourceId, Result};

/// Specification for a job to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Unique identifier for this job.
    pub id: ResourceId,
    /// Container image to run.
    pub image: String,
    /// Command to execute.
    pub command: Vec<String>,
    /// Working directory inside the container.
    pub working_dir: Option<String>,
    /// Environment variables.
    pub env: HashMap<String, String>,
    /// Resource limits.
    pub resources: ResourceRequirements,
    /// Maximum execution time.
    pub timeout: Option<Duration>,
    /// Volumes to mount.
    pub volumes: Vec<VolumeMount>,
    /// Git repository to clone before running commands.
    pub git_clone: Option<GitCloneSpec>,
}

/// Specification for cloning a git repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCloneSpec {
    /// Repository URL to clone.
    pub url: String,
    /// Branch to checkout (defaults to default branch).
    pub branch: Option<String>,
    /// Specific commit SHA to checkout.
    pub sha: Option<String>,
    /// Directory to clone into (defaults to /workspace).
    pub target_dir: String,
    /// Depth for shallow clone (None for full clone).
    pub depth: Option<u32>,
    /// Access token for private repos.
    pub access_token: Option<String>,
}

/// Resource requirements for a job.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    /// CPU limit (e.g., "1000m" for 1 core).
    pub cpu_limit: Option<String>,
    /// Memory limit (e.g., "512Mi").
    pub memory_limit: Option<String>,
    /// CPU request.
    pub cpu_request: Option<String>,
    /// Memory request.
    pub memory_request: Option<String>,
}

/// A volume mount specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Name of the volume.
    pub name: String,
    /// Path to mount in the container.
    pub mount_path: String,
    /// Whether the mount is read-only.
    pub read_only: bool,
}

/// Handle to a running or completed job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHandle {
    /// The job ID.
    pub id: ResourceId,
    /// Executor-specific identifier (e.g., pod name, container ID).
    pub executor_id: String,
    /// Name of the executor running this job.
    pub executor_name: String,
}

/// Status of a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is waiting to start.
    Pending,
    /// Job is currently running.
    Running { started_at: DateTime<Utc> },
    /// Job completed successfully.
    Succeeded {
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    },
    /// Job failed.
    Failed {
        started_at: Option<DateTime<Utc>>,
        finished_at: DateTime<Utc>,
        exit_code: Option<i32>,
        message: String,
    },
    /// Job was cancelled.
    Cancelled {
        started_at: Option<DateTime<Utc>>,
        cancelled_at: DateTime<Utc>,
    },
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobStatus::Succeeded { .. } | JobStatus::Failed { .. } | JobStatus::Cancelled { .. }
        )
    }
}

/// Result of a completed job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    /// Final status.
    pub status: JobStatus,
    /// Exit code if available.
    pub exit_code: Option<i32>,
    /// Artifacts produced by the job.
    pub artifacts: Vec<ArtifactRef>,
}

/// Reference to an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub name: String,
    pub path: String,
    pub size: u64,
}

/// A line of log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub stream: LogStream,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogStream {
    Stdout,
    Stderr,
    System,
}

/// An interactive terminal session.
pub struct TerminalSession {
    pub stdin: Box<dyn futures::Sink<Bytes, Error = std::io::Error> + Send + Unpin>,
    pub stdout: BoxStream<'static, std::result::Result<Bytes, std::io::Error>>,
}

/// Trait for job executors.
#[async_trait]
pub trait Executor: Send + Sync {
    /// Name of this executor.
    fn name(&self) -> &'static str;

    /// Check if this executor can handle the given job spec.
    async fn can_execute(&self, spec: &JobSpec) -> bool;

    /// Spawn a new job.
    async fn spawn(&self, spec: JobSpec) -> Result<JobHandle>;

    /// Get a stream of log lines from a job.
    async fn logs(&self, handle: &JobHandle) -> Result<BoxStream<'static, LogLine>>;

    /// Get the current status of a job.
    async fn status(&self, handle: &JobHandle) -> Result<JobStatus>;

    /// Wait for a job to complete.
    async fn wait(&self, handle: &JobHandle) -> Result<JobResult>;

    /// Cancel a running job.
    async fn cancel(&self, handle: &JobHandle) -> Result<()>;

    /// Open an interactive terminal session to a running job.
    async fn exec_interactive(
        &self,
        handle: &JobHandle,
        cmd: Vec<String>,
    ) -> Result<TerminalSession>;
}
