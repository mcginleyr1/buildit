//! Repository types for connected Git repositories.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Git provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GitProvider {
    Github,
    Gitlab,
    Bitbucket,
}

impl std::fmt::Display for GitProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitProvider::Github => write!(f, "github"),
            GitProvider::Gitlab => write!(f, "gitlab"),
            GitProvider::Bitbucket => write!(f, "bitbucket"),
        }
    }
}

impl std::str::FromStr for GitProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(GitProvider::Github),
            "gitlab" => Ok(GitProvider::Gitlab),
            "bitbucket" => Ok(GitProvider::Bitbucket),
            _ => Err(format!("Unknown git provider: {}", s)),
        }
    }
}

/// Detected configuration files in a repository
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectedConfig {
    /// Path to .buildit.kdl if found
    pub buildit_config: Option<String>,
    /// Paths to Terraform files (.tf)
    pub terraform_files: Vec<String>,
    /// Paths to Terraform directories (containing .tf files)
    pub terraform_dirs: Vec<String>,
    /// Paths to Kubernetes manifest files (.yaml/.yml with k8s resources)
    pub kubernetes_files: Vec<String>,
    /// Paths to directories containing Kubernetes manifests
    pub kubernetes_dirs: Vec<String>,
    /// Dockerfile paths
    pub dockerfiles: Vec<String>,
    /// Helm chart directories (containing Chart.yaml)
    pub helm_charts: Vec<String>,
    /// Other notable files (docker-compose.yml, Makefile, etc.)
    pub other_files: Vec<String>,
}

impl DetectedConfig {
    /// Check if any Terraform configuration was found
    pub fn has_terraform(&self) -> bool {
        !self.terraform_dirs.is_empty()
    }

    /// Check if any Kubernetes configuration was found
    pub fn has_kubernetes(&self) -> bool {
        !self.kubernetes_dirs.is_empty() || !self.helm_charts.is_empty()
    }

    /// Check if a BuildIt pipeline config was found
    pub fn has_pipeline(&self) -> bool {
        self.buildit_config.is_some()
    }

    /// Check if any Dockerfile was found
    pub fn has_dockerfile(&self) -> bool {
        !self.dockerfiles.is_empty()
    }

    /// Summary of what can be created from this config
    pub fn summary(&self) -> Vec<String> {
        let mut items = Vec::new();
        if self.has_terraform() {
            items.push(format!("{} Terraform stack(s)", self.terraform_dirs.len()));
        }
        if self.has_pipeline() {
            items.push("1 Pipeline".to_string());
        }
        if self.has_kubernetes() {
            let count = self.kubernetes_dirs.len() + self.helm_charts.len();
            items.push(format!("{} Application(s)", count));
        }
        if self.has_dockerfile() {
            items.push(format!("{} Dockerfile(s)", self.dockerfiles.len()));
        }
        items
    }
}

/// A connected Git repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub provider: GitProvider,
    pub provider_id: String,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub is_private: bool,
    pub webhook_id: Option<String>,
    pub webhook_secret: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub detected_config: DetectedConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to connect a repository
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectRepositoryRequest {
    pub provider: GitProvider,
    pub owner: String,
    pub name: String,
    /// Optional: personal access token for private repos (if not using OAuth)
    pub access_token: Option<String>,
}

/// Webhook event from a Git provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: Uuid,
    pub repository_id: Option<Uuid>,
    pub provider: GitProvider,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub signature: Option<String>,
    pub signature_valid: Option<bool>,
    pub processed: bool,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Parsed push event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushEvent {
    pub r#ref: String,
    pub before: String,
    pub after: String,
    pub repository_full_name: String,
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub commits: Vec<CommitInfo>,
    pub head_commit: Option<CommitInfo>,
    pub pusher: String,
}

/// Commit information from a push event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
}

impl PushEvent {
    /// Parse a GitHub push webhook payload
    pub fn from_github_payload(payload: &serde_json::Value) -> Option<Self> {
        let r#ref = payload.get("ref")?.as_str()?.to_string();
        let before = payload.get("before")?.as_str()?.to_string();
        let after = payload.get("after")?.as_str()?.to_string();
        let repository_full_name = payload
            .get("repository")?
            .get("full_name")?
            .as_str()?
            .to_string();

        let branch = if r#ref.starts_with("refs/heads/") {
            Some(r#ref.strip_prefix("refs/heads/")?.to_string())
        } else {
            None
        };

        let tag = if r#ref.starts_with("refs/tags/") {
            Some(r#ref.strip_prefix("refs/tags/")?.to_string())
        } else {
            None
        };

        let commits = payload
            .get("commits")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| CommitInfo::from_github_commit(c))
                    .collect()
            })
            .unwrap_or_default();

        let head_commit = payload
            .get("head_commit")
            .and_then(CommitInfo::from_github_commit);

        let pusher = payload
            .get("pusher")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        Some(PushEvent {
            r#ref,
            before,
            after,
            repository_full_name,
            branch,
            tag,
            commits,
            head_commit,
            pusher,
        })
    }
}

impl CommitInfo {
    fn from_github_commit(value: &serde_json::Value) -> Option<Self> {
        Some(CommitInfo {
            sha: value.get("id")?.as_str()?.to_string(),
            message: value.get("message")?.as_str()?.to_string(),
            author: value
                .get("author")
                .and_then(|a| a.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("unknown")
                .to_string(),
            author_email: value
                .get("author")
                .and_then(|a| a.get("email"))
                .and_then(|e| e.as_str())
                .unwrap_or("")
                .to_string(),
            timestamp: value
                .get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                .map(|dt| dt.with_timezone(&Utc)),
            added: value
                .get("added")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            modified: value
                .get("modified")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            removed: value
                .get("removed")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}
