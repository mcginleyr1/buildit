//! Local Docker executor implementation.

use async_trait::async_trait;
use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::HostConfig;
use buildit_core::executor::*;
use buildit_core::{Error, Result};
use chrono::Utc;
use futures::StreamExt;
use futures::stream::BoxStream;
use tracing::{debug, info, warn};

/// Local Docker executor for development and small deployments.
pub struct LocalDockerExecutor {
    docker: Docker,
}

impl LocalDockerExecutor {
    /// Create a new LocalDockerExecutor connecting to the local Docker daemon.
    pub fn new() -> Result<Self> {
        let docker =
            Docker::connect_with_local_defaults().map_err(|e| Error::Internal(e.to_string()))?;
        Ok(Self { docker })
    }

    /// Create with a custom Docker client.
    pub fn with_client(docker: Docker) -> Self {
        Self { docker }
    }

    fn container_name(job_id: &buildit_core::ResourceId) -> String {
        format!("buildit-job-{}", job_id)
    }
}

impl Default for LocalDockerExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to connect to Docker")
    }
}

#[async_trait]
impl Executor for LocalDockerExecutor {
    fn name(&self) -> &'static str {
        "docker"
    }

    async fn can_execute(&self, _spec: &JobSpec) -> bool {
        // Check if Docker is available
        self.docker.ping().await.is_ok()
    }

    async fn spawn(&self, spec: JobSpec) -> Result<JobHandle> {
        let container_name = Self::container_name(&spec.id);

        // Pull the image first
        info!(image = %spec.image, "Pulling image");
        let create_image_options = CreateImageOptions {
            from_image: spec.image.clone(),
            ..Default::default()
        };

        let mut pull_stream = self
            .docker
            .create_image(Some(create_image_options), None, None);
        while let Some(result) = pull_stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        debug!(status = %status, "Pull progress");
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Pull warning");
                }
            }
        }

        // Build environment variables
        let env: Vec<String> = spec
            .env
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // Build the command, prepending git clone if needed
        let cmd = if let Some(ref git_clone) = spec.git_clone {
            // Build git clone command
            let clone_url = if let Some(ref token) = git_clone.access_token {
                if git_clone.url.starts_with("https://") {
                    git_clone
                        .url
                        .replacen("https://", &format!("https://{}@", token), 1)
                } else {
                    git_clone.url.clone()
                }
            } else {
                git_clone.url.clone()
            };

            let depth_arg = git_clone
                .depth
                .map(|d| format!("--depth {}", d))
                .unwrap_or_default();

            let branch_arg = git_clone
                .branch
                .as_ref()
                .map(|b| format!("-b {}", b))
                .unwrap_or_default();

            let checkout_cmd = git_clone
                .sha
                .as_ref()
                .map(|sha| format!(" && git checkout {}", sha))
                .unwrap_or_default();

            let clone_script = format!(
                "git clone {} {} {} {}{}",
                depth_arg, branch_arg, clone_url, &git_clone.target_dir, checkout_cmd
            );

            // Combine clone with original commands
            let user_cmds = spec.command.join(" && ");
            let full_script = if user_cmds.is_empty() {
                clone_script
            } else {
                format!(
                    "{} && cd {} && {}",
                    clone_script, &git_clone.target_dir, user_cmds
                )
            };

            Some(vec!["sh".to_string(), "-c".to_string(), full_script])
        } else if spec.command.is_empty() {
            None
        } else {
            Some(spec.command.clone())
        };

        // Determine working directory
        let working_dir = spec
            .git_clone
            .as_ref()
            .map(|gc| gc.target_dir.clone())
            .or(spec.working_dir.clone());

        // Build volume binds from spec.volumes
        let binds: Option<Vec<String>> = if spec.volumes.is_empty() {
            None
        } else {
            Some(
                spec.volumes
                    .iter()
                    .map(|v| {
                        let mode = if v.read_only { "ro" } else { "rw" };
                        format!("{}:{}:{}", v.name, v.mount_path, mode)
                    })
                    .collect(),
            )
        };

        let host_config = HostConfig {
            binds,
            ..Default::default()
        };

        // Create container config
        let config = Config {
            image: Some(spec.image.clone()),
            cmd,
            env: Some(env),
            working_dir,
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            host_config: Some(host_config),
            ..Default::default()
        };

        let create_options = CreateContainerOptions {
            name: container_name.clone(),
            platform: None,
        };

        // Create the container
        info!(container = %container_name, "Creating container");
        let container = self
            .docker
            .create_container(Some(create_options), config)
            .await
            .map_err(|e| Error::ExecutionFailed(format!("Failed to create container: {}", e)))?;

        // Start the container
        info!(container = %container_name, "Starting container");
        self.docker
            .start_container(&container_name, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| Error::ExecutionFailed(format!("Failed to start container: {}", e)))?;

        Ok(JobHandle {
            id: spec.id,
            executor_id: container.id,
            executor_name: self.name().to_string(),
        })
    }

    async fn logs(&self, handle: &JobHandle) -> Result<BoxStream<'static, LogLine>> {
        let container_name = Self::container_name(&handle.id);

        let options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            timestamps: true,
            ..Default::default()
        };

        let stream = self.docker.logs(&container_name, Some(options));

        let mapped_stream = stream.filter_map(|result| async move {
            match result {
                Ok(output) => {
                    let (stream, content) = match output {
                        LogOutput::StdOut { message } => (
                            LogStream::Stdout,
                            String::from_utf8_lossy(&message).to_string(),
                        ),
                        LogOutput::StdErr { message } => (
                            LogStream::Stderr,
                            String::from_utf8_lossy(&message).to_string(),
                        ),
                        LogOutput::Console { message } => (
                            LogStream::Stdout,
                            String::from_utf8_lossy(&message).to_string(),
                        ),
                        LogOutput::StdIn { message } => (
                            LogStream::Stdout,
                            String::from_utf8_lossy(&message).to_string(),
                        ),
                    };
                    Some(LogLine {
                        timestamp: Utc::now(),
                        stream,
                        content: content.trim_end().to_string(),
                    })
                }
                Err(e) => {
                    warn!(error = %e, "Log stream error");
                    None
                }
            }
        });

        Ok(Box::pin(mapped_stream))
    }

    async fn status(&self, handle: &JobHandle) -> Result<JobStatus> {
        let container_name = Self::container_name(&handle.id);

        let inspect = self
            .docker
            .inspect_container(&container_name, None)
            .await
            .map_err(|e| Error::NotFound(format!("Container not found: {}", e)))?;

        let state = inspect
            .state
            .ok_or_else(|| Error::Internal("No state".to_string()))?;

        let status = if state.running.unwrap_or(false) {
            let started_at = state
                .started_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            JobStatus::Running { started_at }
        } else if state.paused.unwrap_or(false) {
            JobStatus::Pending
        } else {
            // Container has exited
            let exit_code = state.exit_code.map(|c| c as i32);
            let started_at = state
                .started_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc));
            let finished_at = state
                .finished_at
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            if exit_code == Some(0) {
                JobStatus::Succeeded {
                    started_at: started_at.unwrap_or(finished_at),
                    finished_at,
                }
            } else {
                JobStatus::Failed {
                    started_at,
                    finished_at,
                    exit_code,
                    message: state.error.unwrap_or_default(),
                }
            }
        };

        Ok(status)
    }

    async fn wait(&self, handle: &JobHandle) -> Result<JobResult> {
        let container_name = Self::container_name(&handle.id);

        // First check if container is already stopped
        let current_status = self.status(handle).await?;
        if current_status.is_terminal() {
            let exit_code = match &current_status {
                JobStatus::Succeeded { .. } => Some(0),
                JobStatus::Failed { exit_code, .. } => *exit_code,
                _ => None,
            };
            return Ok(JobResult {
                status: current_status,
                exit_code,
                artifacts: vec![],
            });
        }

        // Container is still running, wait for it
        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.docker.wait_container(&container_name, Some(options));

        let exit_code = if let Some(result) = stream.next().await {
            match result {
                Ok(response) => Some(response.status_code as i32),
                Err(e) => {
                    warn!(error = %e, "Wait error");
                    None
                }
            }
        } else {
            None
        };

        let status = self.status(handle).await?;

        Ok(JobResult {
            status,
            exit_code,
            artifacts: vec![], // TODO: Collect artifacts
        })
    }

    async fn cancel(&self, handle: &JobHandle) -> Result<()> {
        let container_name = Self::container_name(&handle.id);

        // Stop the container
        self.docker
            .stop_container(&container_name, None)
            .await
            .map_err(|e| Error::ExecutionFailed(format!("Failed to stop container: {}", e)))?;

        // Remove the container
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        self.docker
            .remove_container(&container_name, Some(options))
            .await
            .map_err(|e| Error::ExecutionFailed(format!("Failed to remove container: {}", e)))?;

        Ok(())
    }

    async fn exec_interactive(
        &self,
        _handle: &JobHandle,
        _cmd: Vec<String>,
    ) -> Result<TerminalSession> {
        // TODO: Implement interactive exec for Docker
        // This is complex and not needed for MVP
        Err(Error::Internal(
            "Interactive exec not yet implemented for Docker".to_string(),
        ))
    }
}

/// Cleanup a job's container.
pub async fn cleanup_container(docker: &Docker, job_id: &buildit_core::ResourceId) -> Result<()> {
    let container_name = LocalDockerExecutor::container_name(job_id);

    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };

    docker
        .remove_container(&container_name, Some(options))
        .await
        .map_err(|e| Error::ExecutionFailed(format!("Failed to remove container: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildit_core::executor::ResourceRequirements;
    use std::collections::HashMap;

    fn make_test_spec() -> JobSpec {
        JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "hello".to_string()],
            working_dir: Some("/workspace".to_string()),
            env: {
                let mut env = HashMap::new();
                env.insert("FOO".to_string(), "bar".to_string());
                env
            },
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        }
    }

    #[test]
    fn test_container_name_generation() {
        let id = buildit_core::ResourceId::new();
        let name = LocalDockerExecutor::container_name(&id);

        assert!(name.starts_with("buildit-job-"));
        assert!(name.len() > 12); // "buildit-job-" + UUID
    }

    #[test]
    fn test_container_name_is_deterministic() {
        let id = buildit_core::ResourceId::new();
        let name1 = LocalDockerExecutor::container_name(&id);
        let name2 = LocalDockerExecutor::container_name(&id);
        assert_eq!(name1, name2);
    }

    #[test]
    fn test_container_name_unique_per_id() {
        let id1 = buildit_core::ResourceId::new();
        let id2 = buildit_core::ResourceId::new();
        let name1 = LocalDockerExecutor::container_name(&id1);
        let name2 = LocalDockerExecutor::container_name(&id2);
        assert_ne!(name1, name2);
    }

    #[test]
    fn test_job_spec_structure() {
        let spec = make_test_spec();

        assert_eq!(spec.image, "alpine:latest");
        assert_eq!(spec.command, vec!["echo", "hello"]);
        assert_eq!(spec.working_dir, Some("/workspace".to_string()));
        assert_eq!(spec.env.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn test_empty_command_spec() {
        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        assert!(spec.command.is_empty());
        assert!(spec.working_dir.is_none());
    }

    #[test]
    fn test_job_handle_structure() {
        let id = buildit_core::ResourceId::new();
        let handle = JobHandle {
            id: id.clone(),
            executor_id: "container-abc123".to_string(),
            executor_name: "docker".to_string(),
        };

        assert_eq!(handle.id, id);
        assert_eq!(handle.executor_id, "container-abc123");
        assert_eq!(handle.executor_name, "docker");
    }

    #[test]
    fn test_job_status_variants() {
        // Test Pending
        let pending = JobStatus::Pending;
        assert!(!pending.is_terminal());

        // Test Running
        let running = JobStatus::Running {
            started_at: chrono::Utc::now(),
        };
        assert!(!running.is_terminal());

        // Test Succeeded
        let succeeded = JobStatus::Succeeded {
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
        };
        assert!(succeeded.is_terminal());

        // Test Failed
        let failed = JobStatus::Failed {
            started_at: Some(chrono::Utc::now()),
            finished_at: chrono::Utc::now(),
            exit_code: Some(1),
            message: "Command failed".to_string(),
        };
        assert!(failed.is_terminal());

        // Test Cancelled
        let cancelled = JobStatus::Cancelled {
            started_at: Some(chrono::Utc::now()),
            cancelled_at: chrono::Utc::now(),
        };
        assert!(cancelled.is_terminal());
    }

    #[test]
    fn test_log_line_structure() {
        let log = LogLine {
            timestamp: chrono::Utc::now(),
            stream: LogStream::Stdout,
            content: "Hello, World!".to_string(),
        };

        assert_eq!(log.content, "Hello, World!");
        assert!(matches!(log.stream, LogStream::Stdout));
    }

    #[test]
    fn test_log_stream_variants() {
        let stdout = LogStream::Stdout;
        let stderr = LogStream::Stderr;
        let system = LogStream::System;

        // Just verify they exist and are different
        assert!(matches!(stdout, LogStream::Stdout));
        assert!(matches!(stderr, LogStream::Stderr));
        assert!(matches!(system, LogStream::System));
    }

    #[test]
    fn test_job_result_structure() {
        let result = JobResult {
            status: JobStatus::Succeeded {
                started_at: chrono::Utc::now(),
                finished_at: chrono::Utc::now(),
            },
            exit_code: Some(0),
            artifacts: vec![],
        };

        assert_eq!(result.exit_code, Some(0));
        assert!(result.artifacts.is_empty());
        assert!(result.status.is_terminal());
    }
}

/// Integration tests that require Docker to be running.
/// Run with: cargo test -- --ignored
#[cfg(test)]
mod integration_tests {
    use super::*;
    use buildit_core::executor::ResourceRequirements;
    use std::collections::HashMap;

    /// Test that we can create an executor when Docker is available.
    #[tokio::test]
    #[ignore]
    async fn test_executor_creation() {
        let executor = LocalDockerExecutor::new();
        assert!(executor.is_ok(), "Should connect to Docker daemon");

        let executor = executor.unwrap();
        assert_eq!(executor.name(), "docker");
    }

    /// Test can_execute returns true when Docker is running.
    #[tokio::test]
    #[ignore]
    async fn test_can_execute() {
        let executor = LocalDockerExecutor::new().unwrap();

        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec!["echo".to_string(), "test".to_string()],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let can_execute = executor.can_execute(&spec).await;
        assert!(
            can_execute,
            "Should be able to execute when Docker is running"
        );
    }

    /// Test full job lifecycle: spawn, wait, check result.
    #[tokio::test]
    #[ignore]
    async fn test_job_lifecycle() {
        let executor = LocalDockerExecutor::new().unwrap();

        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo 'Hello from Docker!'".to_string(),
            ],
            working_dir: None,
            env: {
                let mut env = HashMap::new();
                env.insert("TEST_VAR".to_string(), "test_value".to_string());
                env
            },
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        // Spawn the job
        let handle = executor.spawn(spec).await.expect("Should spawn container");
        assert_eq!(handle.executor_name, "docker");

        // Wait for completion
        let result = executor
            .wait(&handle)
            .await
            .expect("Should wait for container");

        // Check it succeeded
        assert_eq!(result.exit_code, Some(0));
        match result.status {
            JobStatus::Succeeded { .. } => {}
            other => panic!("Expected Succeeded, got {:?}", other),
        }

        // Cleanup
        let _ = executor.cancel(&handle).await;
    }

    /// Test that a failing job reports failure correctly.
    #[tokio::test]
    #[ignore]
    async fn test_failing_job() {
        let executor = LocalDockerExecutor::new().unwrap();

        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "exit 42".to_string(),
            ],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn container");
        let result = executor
            .wait(&handle)
            .await
            .expect("Should wait for container");

        assert_eq!(result.exit_code, Some(42));
        match result.status {
            JobStatus::Failed { exit_code, .. } => {
                assert_eq!(exit_code, Some(42));
            }
            other => panic!("Expected Failed, got {:?}", other),
        }

        // Cleanup
        let _ = executor.cancel(&handle).await;
    }

    /// Test job cancellation.
    #[tokio::test]
    #[ignore]
    async fn test_job_cancellation() {
        let executor = LocalDockerExecutor::new().unwrap();

        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "sleep 300".to_string(),
            ],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn container");

        // Give it a moment to start
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Check it's running
        let status = executor.status(&handle).await.expect("Should get status");
        assert!(
            matches!(status, JobStatus::Running { .. }),
            "Should be running"
        );

        // Cancel it
        executor
            .cancel(&handle)
            .await
            .expect("Should cancel container");

        // Verify it's gone
        let status_result = executor.status(&handle).await;
        assert!(status_result.is_err(), "Container should be removed");
    }

    /// Test log streaming from a job.
    #[tokio::test]
    #[ignore]
    async fn test_log_streaming() {
        let executor = LocalDockerExecutor::new().unwrap();

        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo 'line1'; echo 'line2'; echo 'line3'".to_string(),
            ],
            working_dir: None,
            env: HashMap::new(),
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn container");

        // Wait for job to complete first
        let _ = executor.wait(&handle).await;

        // Get logs
        let mut log_stream = executor.logs(&handle).await.expect("Should get logs");

        // Collect logs
        let mut logs = Vec::new();
        while let Some(log_line) = log_stream.next().await {
            logs.push(log_line.content);
        }

        // Verify we got expected output
        assert!(
            logs.iter().any(|l| l.contains("line1")),
            "Should have line1"
        );
        assert!(
            logs.iter().any(|l| l.contains("line2")),
            "Should have line2"
        );
        assert!(
            logs.iter().any(|l| l.contains("line3")),
            "Should have line3"
        );

        // Cleanup
        let _ = executor.cancel(&handle).await;
    }

    /// Test environment variable injection.
    #[tokio::test]
    #[ignore]
    async fn test_environment_variables() {
        let executor = LocalDockerExecutor::new().unwrap();

        let spec = JobSpec {
            id: buildit_core::ResourceId::new(),
            image: "alpine:latest".to_string(),
            command: vec![
                "/bin/sh".to_string(),
                "-c".to_string(),
                "echo $MY_VAR".to_string(),
            ],
            working_dir: None,
            env: {
                let mut env = HashMap::new();
                env.insert("MY_VAR".to_string(), "hello_world".to_string());
                env
            },
            resources: ResourceRequirements::default(),
            timeout: None,
            volumes: vec![],
        };

        let handle = executor.spawn(spec).await.expect("Should spawn container");
        let _ = executor.wait(&handle).await;

        let mut log_stream = executor.logs(&handle).await.expect("Should get logs");
        let mut found = false;
        while let Some(log_line) = log_stream.next().await {
            if log_line.content.contains("hello_world") {
                found = true;
                break;
            }
        }

        assert!(found, "Should find environment variable in output");

        let _ = executor.cancel(&handle).await;
    }
}
