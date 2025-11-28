//! Pipeline orchestrator - executes pipeline stages in dependency order.

use buildit_config::VariableContext;
use buildit_core::ResourceId;
use buildit_core::executor::{
    Executor, JobSpec, JobStatus, LogLine, ResourceRequirements, VolumeMount,
};
use buildit_core::pipeline::{Pipeline, Stage, StageAction};
use futures::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

/// State of a stage during execution.
#[derive(Debug, Clone)]
pub enum StageState {
    Pending,
    Running { job_id: ResourceId },
    Succeeded,
    Failed { message: String },
    Skipped { reason: String },
}

impl StageState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StageState::Succeeded | StageState::Failed { .. } | StageState::Skipped { .. }
        )
    }

    pub fn is_success(&self) -> bool {
        matches!(self, StageState::Succeeded)
    }
}

/// Event emitted during pipeline execution.
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    StageStarted { stage: String },
    StageLog { stage: String, line: LogLine },
    StageCompleted { stage: String, success: bool },
    PipelineCompleted { success: bool },
}

/// Result of a pipeline execution.
#[derive(Debug)]
pub struct PipelineResult {
    pub success: bool,
    pub stage_states: HashMap<String, StageState>,
}

/// Orchestrates the execution of a pipeline.
pub struct PipelineOrchestrator {
    executor: Arc<dyn Executor>,
    /// Working directory to mount into containers
    working_dir: Option<PathBuf>,
}

impl PipelineOrchestrator {
    pub fn new(executor: Arc<dyn Executor>) -> Self {
        Self {
            executor,
            working_dir: None,
        }
    }

    /// Create an orchestrator with a working directory to mount into containers.
    pub fn with_working_dir(executor: Arc<dyn Executor>, working_dir: PathBuf) -> Self {
        Self {
            executor,
            working_dir: Some(working_dir),
        }
    }

    /// Execute a pipeline, returning a channel of events and a handle to get the final result.
    ///
    /// The `var_ctx` provides variable interpolation for commands and environment variables.
    /// Variables like `${git.sha}`, `${git.branch}`, `${env.VAR}`, etc. will be substituted.
    pub fn execute(
        &self,
        pipeline: &Pipeline,
        env: HashMap<String, String>,
        var_ctx: Option<VariableContext>,
    ) -> (
        mpsc::Receiver<PipelineEvent>,
        tokio::task::JoinHandle<PipelineResult>,
    ) {
        let (tx, rx) = mpsc::channel(100);
        let executor = self.executor.clone();
        let working_dir = self.working_dir.clone();
        let stages = pipeline.stages.clone();
        let var_ctx = var_ctx.unwrap_or_default();

        let handle = tokio::spawn(async move {
            Self::execute_inner(executor, working_dir, stages, env, var_ctx, tx).await
        });

        (rx, handle)
    }

    /// Internal execution logic
    async fn execute_inner(
        executor: Arc<dyn Executor>,
        working_dir: Option<PathBuf>,
        stages: Vec<Stage>,
        env: HashMap<String, String>,
        mut var_ctx: VariableContext,
        tx: mpsc::Sender<PipelineEvent>,
    ) -> PipelineResult {
        let mut stage_states: HashMap<String, StageState> = stages
            .iter()
            .map(|s| (s.name.clone(), StageState::Pending))
            .collect();

        // Build execution order using topological sort
        let execution_order = Self::topological_sort(&stages);

        for (stage_idx, stage_name) in execution_order.iter().enumerate() {
            let stage = stages.iter().find(|s| s.name == *stage_name).unwrap();

            // Update stage context for variable interpolation
            var_ctx.stage.name = stage.name.clone();
            var_ctx.stage.index = stage_idx;

            // Check if dependencies are satisfied
            let deps_satisfied = stage.needs.iter().all(|dep| {
                stage_states
                    .get(dep)
                    .map(|s| s.is_success())
                    .unwrap_or(false)
            });

            if !deps_satisfied {
                let failed_deps: Vec<_> = stage
                    .needs
                    .iter()
                    .filter(|dep| {
                        !stage_states
                            .get(*dep)
                            .map(|s| s.is_success())
                            .unwrap_or(false)
                    })
                    .collect();
                info!(stage = %stage.name, ?failed_deps, "Skipping stage due to failed dependencies");
                stage_states.insert(
                    stage.name.clone(),
                    StageState::Skipped {
                        reason: format!("Dependencies failed: {:?}", failed_deps),
                    },
                );
                continue;
            }

            // Check conditional execution
            if let Some(condition) = &stage.when {
                // TODO: Implement condition evaluation
                // For now, we'll just run all stages
                let _ = condition;
            }

            // Execute the stage
            let _ = tx
                .send(PipelineEvent::StageStarted {
                    stage: stage.name.clone(),
                })
                .await;

            match Self::execute_stage(&executor, &working_dir, stage, &env, &var_ctx, &tx).await {
                Ok(()) => {
                    info!(stage = %stage.name, "Stage completed successfully");
                    stage_states.insert(stage.name.clone(), StageState::Succeeded);
                    let _ = tx
                        .send(PipelineEvent::StageCompleted {
                            stage: stage.name.clone(),
                            success: true,
                        })
                        .await;
                }
                Err(e) => {
                    error!(stage = %stage.name, error = %e, "Stage failed");
                    stage_states.insert(
                        stage.name.clone(),
                        StageState::Failed {
                            message: e.to_string(),
                        },
                    );
                    let _ = tx
                        .send(PipelineEvent::StageCompleted {
                            stage: stage.name.clone(),
                            success: false,
                        })
                        .await;
                }
            }
        }

        let success = stage_states.values().all(|s| s.is_success());
        let _ = tx.send(PipelineEvent::PipelineCompleted { success }).await;

        PipelineResult {
            success,
            stage_states,
        }
    }

    /// Execute a single stage.
    async fn execute_stage(
        executor: &Arc<dyn Executor>,
        working_dir: &Option<PathBuf>,
        stage: &Stage,
        env: &HashMap<String, String>,
        var_ctx: &VariableContext,
        tx: &mpsc::Sender<PipelineEvent>,
    ) -> Result<(), String> {
        match &stage.action {
            StageAction::Run {
                image,
                commands,
                artifacts: _,
            } => {
                // Combine global env with stage env
                let mut full_env = env.clone();
                full_env.extend(stage.env.clone());

                // Apply variable interpolation to environment values
                let full_env = var_ctx.interpolate_map(&full_env);

                // Apply variable interpolation to commands
                let interpolated_commands = var_ctx.interpolate_vec(commands);

                // Apply variable interpolation to image
                let interpolated_image = var_ctx.interpolate(image);

                // Build the job spec
                // We'll run commands as a shell script
                let script = interpolated_commands.join(" && ");
                let command = vec!["/bin/sh".to_string(), "-c".to_string(), script];

                // Build volume mounts - mount working directory if provided
                let volumes = if let Some(wd) = working_dir {
                    vec![VolumeMount {
                        name: wd.to_string_lossy().to_string(),
                        mount_path: "/workspace".to_string(),
                        read_only: false,
                    }]
                } else {
                    vec![]
                };

                let job_spec = JobSpec {
                    id: ResourceId::new(),
                    image: interpolated_image.clone(),
                    command,
                    working_dir: Some("/workspace".to_string()),
                    env: full_env,
                    resources: ResourceRequirements::default(),
                    timeout: None,
                    volumes,
                };

                info!(stage = %stage.name, image = %interpolated_image, "Spawning job");

                // Spawn the job
                let handle = executor
                    .spawn(job_spec)
                    .await
                    .map_err(|e| format!("Failed to spawn job: {}", e))?;

                // Stream logs
                let log_stream = executor
                    .logs(&handle)
                    .await
                    .map_err(|e| format!("Failed to get logs: {}", e))?;

                let stage_name = stage.name.clone();
                let tx_clone = tx.clone();

                // Spawn a task to stream logs
                let log_handle = tokio::spawn(async move {
                    let mut stream = log_stream;
                    while let Some(line) = stream.next().await {
                        let _ = tx_clone
                            .send(PipelineEvent::StageLog {
                                stage: stage_name.clone(),
                                line,
                            })
                            .await;
                    }
                });

                // Wait for job completion
                let result = executor
                    .wait(&handle)
                    .await
                    .map_err(|e| format!("Failed to wait for job: {}", e))?;

                // Abort log streaming task (it may still be following a stopped container)
                log_handle.abort();
                let _ = log_handle.await;

                // Check result
                match result.status {
                    JobStatus::Succeeded { .. } => Ok(()),
                    JobStatus::Failed { message, .. } => Err(format!("Job failed: {}", message)),
                    JobStatus::Cancelled { .. } => Err("Job was cancelled".to_string()),
                    _ => Err("Job ended in unexpected state".to_string()),
                }
            }
            StageAction::ImageBuild { .. } => {
                // TODO: Implement image building
                Err("Image build not yet implemented".to_string())
            }
            StageAction::Deploy(_) => {
                // TODO: Implement deployment
                Err("Deploy not yet implemented".to_string())
            }
            StageAction::Parallel { .. } => {
                // TODO: Implement parallel execution
                Err("Parallel stages not yet implemented".to_string())
            }
            StageAction::Matrix { .. } => {
                // TODO: Implement matrix builds
                Err("Matrix builds not yet implemented".to_string())
            }
        }
    }

    /// Topological sort of stages based on dependencies.
    fn topological_sort(stages: &[Stage]) -> Vec<String> {
        let mut result = Vec::new();
        let mut visited = HashMap::new();
        let stage_map: HashMap<&str, &Stage> =
            stages.iter().map(|s| (s.name.as_str(), s)).collect();

        for stage in stages {
            Self::topo_visit(&stage.name, &stage_map, &mut visited, &mut result);
        }

        result
    }

    fn topo_visit(
        name: &str,
        stage_map: &HashMap<&str, &Stage>,
        visited: &mut HashMap<String, bool>,
        result: &mut Vec<String>,
    ) {
        if visited.get(name).copied().unwrap_or(false) {
            return;
        }

        visited.insert(name.to_string(), true);

        if let Some(stage) = stage_map.get(name) {
            for dep in &stage.needs {
                Self::topo_visit(dep, stage_map, visited, result);
            }
        }

        result.push(name.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use buildit_core::pipeline::StageAction;

    fn make_stage(name: &str, needs: Vec<&str>) -> Stage {
        Stage {
            name: name.to_string(),
            needs: needs.into_iter().map(String::from).collect(),
            when: None,
            manual: false,
            action: StageAction::Run {
                image: "alpine".to_string(),
                commands: vec!["echo hello".to_string()],
                artifacts: vec![],
            },
            env: HashMap::new(),
        }
    }

    #[test]
    fn test_topological_sort() {
        let stages = vec![
            make_stage("deploy", vec!["build"]),
            make_stage("test", vec![]),
            make_stage("build", vec!["test"]),
        ];

        let order = PipelineOrchestrator::topological_sort(&stages);

        // test should come before build, build should come before deploy
        let test_idx = order.iter().position(|s| s == "test").unwrap();
        let build_idx = order.iter().position(|s| s == "build").unwrap();
        let deploy_idx = order.iter().position(|s| s == "deploy").unwrap();

        assert!(test_idx < build_idx);
        assert!(build_idx < deploy_idx);
    }

    struct MockExecutor;

    #[async_trait::async_trait]
    impl Executor for MockExecutor {
        fn name(&self) -> &'static str {
            "mock"
        }

        async fn can_execute(&self, _spec: &JobSpec) -> bool {
            true
        }

        async fn spawn(
            &self,
            _spec: JobSpec,
        ) -> buildit_core::Result<buildit_core::executor::JobHandle> {
            unimplemented!()
        }

        async fn logs(
            &self,
            _handle: &buildit_core::executor::JobHandle,
        ) -> buildit_core::Result<futures::stream::BoxStream<'static, LogLine>> {
            unimplemented!()
        }

        async fn status(
            &self,
            _handle: &buildit_core::executor::JobHandle,
        ) -> buildit_core::Result<JobStatus> {
            unimplemented!()
        }

        async fn wait(
            &self,
            _handle: &buildit_core::executor::JobHandle,
        ) -> buildit_core::Result<buildit_core::executor::JobResult> {
            unimplemented!()
        }

        async fn cancel(
            &self,
            _handle: &buildit_core::executor::JobHandle,
        ) -> buildit_core::Result<()> {
            unimplemented!()
        }

        async fn exec_interactive(
            &self,
            _handle: &buildit_core::executor::JobHandle,
            _cmd: Vec<String>,
        ) -> buildit_core::Result<buildit_core::executor::TerminalSession> {
            unimplemented!()
        }
    }
}
