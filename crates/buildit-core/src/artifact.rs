//! Artifact storage abstraction.

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use crate::{ResourceId, Result};

/// Key for storing/retrieving an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactKey {
    /// Pipeline run ID.
    pub run_id: ResourceId,
    /// Stage name.
    pub stage: String,
    /// Artifact name/path.
    pub name: String,
}

/// Reference to a stored artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Storage key.
    pub key: ArtifactKey,
    /// Storage location (backend-specific).
    pub location: String,
    /// Content hash for integrity.
    pub checksum: String,
    /// Size in bytes.
    pub size: u64,
    /// When the artifact was stored.
    pub created_at: DateTime<Utc>,
}

/// Metadata about an artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManifest {
    pub reference: ArtifactRef,
    /// MIME type if known.
    pub content_type: Option<String>,
    /// Custom metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Policy for artifact retention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum age of artifacts to keep.
    pub max_age: Option<std::time::Duration>,
    /// Maximum total size per pipeline.
    pub max_size_bytes: Option<u64>,
    /// Minimum number of runs to keep artifacts for.
    pub min_runs: Option<u32>,
}

/// Statistics from a prune operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneStats {
    pub artifacts_deleted: u64,
    pub bytes_freed: u64,
}

/// Trait for artifact storage backends.
#[async_trait]
pub trait ArtifactStore: Send + Sync {
    /// Store an artifact.
    async fn put(&self, key: &ArtifactKey, data: Bytes) -> Result<ArtifactRef>;

    /// Retrieve an artifact.
    async fn get(&self, reference: &ArtifactRef) -> Result<Bytes>;

    /// Stream an artifact (for large files).
    async fn stream(
        &self,
        reference: &ArtifactRef,
    ) -> Result<BoxStream<'static, std::result::Result<Bytes, std::io::Error>>>;

    /// List artifacts for a pipeline run.
    async fn list(&self, run_id: &ResourceId) -> Result<Vec<ArtifactManifest>>;

    /// Delete an artifact.
    async fn delete(&self, reference: &ArtifactRef) -> Result<()>;

    /// Prune artifacts according to a policy.
    async fn prune(&self, policy: RetentionPolicy) -> Result<PruneStats>;
}
