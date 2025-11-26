//! Local Docker executor implementation.

use async_trait::async_trait;
use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::image::CreateImageOptions;
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

        // Build the command
        // If command is empty, use shell to run nothing (container will use default)
        let cmd = if spec.command.is_empty() {
            None
        } else {
            Some(spec.command.clone())
        };

        // Create container config
        let config = Config {
            image: Some(spec.image.clone()),
            cmd,
            env: Some(env),
            working_dir: spec.working_dir.clone(),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
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
