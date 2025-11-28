//! Git service for cloning repositories and detecting configuration files.

use buildit_core::repository::DetectedConfig;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, info, warn};

/// Service for Git operations.
pub struct GitService {
    /// Base directory for cloned repositories
    work_dir: PathBuf,
}

impl Default for GitService {
    fn default() -> Self {
        Self::new()
    }
}

impl GitService {
    pub fn new() -> Self {
        let work_dir = std::env::var("BUILDIT_WORK_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("buildit-repos"));

        Self { work_dir }
    }

    /// Clone a repository and scan for configuration files.
    pub async fn clone_and_scan(
        &self,
        clone_url: &str,
        access_token: Option<&str>,
    ) -> Result<DetectedConfig, GitError> {
        // Create work directory if it doesn't exist
        tokio::fs::create_dir_all(&self.work_dir).await?;

        // Generate a unique directory name for this clone
        let repo_hash = format!("{:x}", md5::compute(clone_url));
        let clone_path = self.work_dir.join(&repo_hash);

        // Remove existing clone if present
        if clone_path.exists() {
            tokio::fs::remove_dir_all(&clone_path).await?;
        }

        // Build clone URL with authentication if provided
        let auth_url = if let Some(token) = access_token {
            // Insert token into URL for authentication
            // https://github.com/owner/repo.git -> https://token@github.com/owner/repo.git
            if let Some(rest) = clone_url.strip_prefix("https://") {
                format!("https://{}@{}", token, rest)
            } else {
                clone_url.to_string()
            }
        } else {
            clone_url.to_string()
        };

        info!(clone_url = %clone_url, path = %clone_path.display(), "Cloning repository");

        // Clone the repository (shallow clone for speed)
        let output = Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--single-branch",
                &auth_url,
                clone_path.to_str().unwrap(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't log the full error as it may contain the token
            warn!("Git clone failed");
            return Err(GitError::CloneFailed(
                stderr.replace(access_token.unwrap_or(""), "[REDACTED]"),
            ));
        }

        info!(path = %clone_path.display(), "Repository cloned successfully");

        // Scan for configuration files
        let detected_config = self.scan_repository(&clone_path).await?;

        info!(
            buildit_config = ?detected_config.buildit_config,
            terraform_dirs = ?detected_config.terraform_dirs,
            "Configuration files detected"
        );

        Ok(detected_config)
    }

    /// Scan a repository directory for configuration files.
    async fn scan_repository(&self, repo_path: &Path) -> Result<DetectedConfig, GitError> {
        let mut config = DetectedConfig::default();

        // Walk the directory tree
        self.scan_directory(repo_path, repo_path, &mut config)
            .await?;

        // Deduplicate terraform_dirs based on unique directories containing .tf files
        config.terraform_dirs.sort();
        config.terraform_dirs.dedup();

        Ok(config)
    }

    /// Recursively scan a directory.
    #[async_recursion::async_recursion]
    async fn scan_directory(
        &self,
        base_path: &Path,
        current_path: &Path,
        config: &mut DetectedConfig,
    ) -> Result<(), GitError> {
        let mut entries = tokio::fs::read_dir(current_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip hidden directories (like .git)
            if file_name_str.starts_with('.') && path.is_dir() {
                continue;
            }

            if path.is_dir() {
                // Skip common non-source directories
                if matches!(
                    file_name_str.as_ref(),
                    "node_modules" | "target" | "vendor" | ".terraform" | "__pycache__"
                ) {
                    continue;
                }

                // Recurse into subdirectory
                self.scan_directory(base_path, &path, config).await?;
            } else if path.is_file() {
                let relative_path = path
                    .strip_prefix(base_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                // Check for .buildit.kdl
                if file_name_str == ".buildit.kdl" || file_name_str == "buildit.kdl" {
                    debug!(path = %relative_path, "Found BuildIt config");
                    config.buildit_config = Some(relative_path.clone());
                }

                // Check for Terraform files
                if file_name_str.ends_with(".tf") {
                    debug!(path = %relative_path, "Found Terraform file");
                    config.terraform_files.push(relative_path.clone());

                    // Add the directory containing this .tf file
                    if let Some(parent) = path.parent() {
                        let parent_relative = parent
                            .strip_prefix(base_path)
                            .unwrap_or(parent)
                            .to_string_lossy()
                            .to_string();
                        let dir = if parent_relative.is_empty() {
                            ".".to_string()
                        } else {
                            parent_relative
                        };
                        if !config.terraform_dirs.contains(&dir) {
                            config.terraform_dirs.push(dir);
                        }
                    }
                }

                // Check for Dockerfiles
                if file_name_str == "Dockerfile" || file_name_str.starts_with("Dockerfile.") {
                    debug!(path = %relative_path, "Found Dockerfile");
                    config.dockerfiles.push(relative_path.clone());
                }

                // Check for Helm charts (Chart.yaml)
                if file_name_str == "Chart.yaml" {
                    debug!(path = %relative_path, "Found Helm chart");
                    if let Some(parent) = path.parent() {
                        let parent_relative = parent
                            .strip_prefix(base_path)
                            .unwrap_or(parent)
                            .to_string_lossy()
                            .to_string();
                        let dir = if parent_relative.is_empty() {
                            ".".to_string()
                        } else {
                            parent_relative
                        };
                        if !config.helm_charts.contains(&dir) {
                            config.helm_charts.push(dir);
                        }
                    }
                }

                // Check for Kubernetes manifests (.yaml/.yml files with k8s content)
                if (file_name_str.ends_with(".yaml") || file_name_str.ends_with(".yml"))
                    && !file_name_str.starts_with(".")
                {
                    // Read file and check if it looks like a K8s manifest
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        if Self::looks_like_k8s_manifest(&content) {
                            debug!(path = %relative_path, "Found Kubernetes manifest");
                            config.kubernetes_files.push(relative_path.clone());

                            // Add the directory
                            if let Some(parent) = path.parent() {
                                let parent_relative = parent
                                    .strip_prefix(base_path)
                                    .unwrap_or(parent)
                                    .to_string_lossy()
                                    .to_string();
                                let dir = if parent_relative.is_empty() {
                                    ".".to_string()
                                } else {
                                    parent_relative
                                };
                                if !config.kubernetes_dirs.contains(&dir) {
                                    config.kubernetes_dirs.push(dir);
                                }
                            }
                        }
                    }
                }

                // Check for other notable files
                if matches!(
                    file_name_str.as_ref(),
                    "docker-compose.yml"
                        | "docker-compose.yaml"
                        | "Makefile"
                        | "Cargo.toml"
                        | "package.json"
                        | "go.mod"
                        | "requirements.txt"
                        | "Gemfile"
                        | "Kustomization.yaml"
                        | "kustomization.yaml"
                ) {
                    config.other_files.push(relative_path);
                }
            }
        }

        Ok(())
    }

    /// Check if YAML content looks like a Kubernetes manifest.
    fn looks_like_k8s_manifest(content: &str) -> bool {
        // Look for common K8s resource indicators
        let k8s_indicators = [
            "apiVersion:",
            "kind: Deployment",
            "kind: Service",
            "kind: ConfigMap",
            "kind: Secret",
            "kind: Ingress",
            "kind: StatefulSet",
            "kind: DaemonSet",
            "kind: Job",
            "kind: CronJob",
            "kind: Pod",
            "kind: Namespace",
            "kind: ServiceAccount",
            "kind: Role",
            "kind: RoleBinding",
            "kind: ClusterRole",
            "kind: ClusterRoleBinding",
            "kind: PersistentVolumeClaim",
            "kind: PersistentVolume",
            "kind: HorizontalPodAutoscaler",
            "kind: NetworkPolicy",
        ];

        // Must have apiVersion and at least look like a K8s resource
        content.contains("apiVersion:")
            && k8s_indicators
                .iter()
                .any(|indicator| content.contains(indicator))
    }

    /// Get the local path for a cloned repository.
    pub fn get_repo_path(&self, clone_url: &str) -> PathBuf {
        let repo_hash = format!("{:x}", md5::compute(clone_url));
        self.work_dir.join(repo_hash)
    }

    /// Ensure a repository is cloned and up-to-date, return its path.
    pub async fn ensure_cloned(
        &self,
        clone_url: &str,
        access_token: Option<&str>,
    ) -> Result<PathBuf, GitError> {
        let repo_path = self.get_repo_path(clone_url);

        if repo_path.exists() {
            // Pull latest changes
            info!(path = %repo_path.display(), "Pulling latest changes");
            let output = Command::new("git")
                .args(["pull", "--ff-only"])
                .current_dir(&repo_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await?;

            if !output.status.success() {
                // If pull fails, re-clone
                warn!("Git pull failed, re-cloning");
                self.clone_and_scan(clone_url, access_token).await?;
            }
        } else {
            // Clone fresh
            self.clone_and_scan(clone_url, access_token).await?;
        }

        Ok(repo_path)
    }
}

/// Git operation errors.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Clone failed: {0}")]
    CloneFailed(String),

    #[error("Invalid repository URL")]
    InvalidUrl,
}
