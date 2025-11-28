//! Stack runner service for executing Terraform operations in containers.
//!
//! This service orchestrates:
//! 1. Cloning repository with Terraform code
//! 2. Setting up workspace with variables
//! 3. Running terraform init/plan/apply in containers
//! 4. Streaming output and updating run status

use buildit_core::ResourceId;
use buildit_core::executor::{JobSpec, ResourceRequirements};
use buildit_core::stack::{StackRunStatus, StackRunType};
use buildit_db::{PgStackRepo, StackRepo};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};

use crate::services::git::GitService;

/// Configuration for the stack runner.
#[derive(Clone)]
pub struct StackRunnerConfig {
    /// Terraform Docker image to use
    pub terraform_image: String,
    /// Working directory base path for cloned repos
    pub work_dir: String,
    /// Namespace for Kubernetes jobs (if using K8s executor)
    pub namespace: String,
}

impl Default for StackRunnerConfig {
    fn default() -> Self {
        Self {
            terraform_image: std::env::var("TERRAFORM_IMAGE")
                .unwrap_or_else(|_| "hashicorp/terraform:1.5".to_string()),
            work_dir: std::env::var("STACK_WORK_DIR")
                .unwrap_or_else(|_| "/tmp/buildit/stacks".to_string()),
            namespace: std::env::var("BUILDIT_JOB_NAMESPACE")
                .unwrap_or_else(|_| "buildit".to_string()),
        }
    }
}

/// Service for running Terraform stacks.
pub struct StackRunner {
    config: StackRunnerConfig,
    stack_repo: Arc<PgStackRepo>,
    git_service: GitService,
}

impl StackRunner {
    pub fn new(config: StackRunnerConfig, stack_repo: Arc<PgStackRepo>) -> Self {
        Self {
            config,
            stack_repo,
            git_service: GitService::new(),
        }
    }

    /// Execute a stack run (plan, apply, or destroy).
    pub async fn execute_run(&self, run_id: ResourceId) -> Result<(), StackRunnerError> {
        // Get run details
        let run = self
            .stack_repo
            .get_run(run_id)
            .await
            .map_err(|e| StackRunnerError::Database(e.to_string()))?;

        let stack = self
            .stack_repo
            .get_stack(ResourceId::from_uuid(run.stack_id))
            .await
            .map_err(|e| StackRunnerError::Database(e.to_string()))?;

        info!(
            run_id = %run.id,
            stack = %stack.name,
            run_type = ?run.run_type,
            "Starting stack run"
        );

        // Update status to running
        self.stack_repo
            .update_run_started(run_id)
            .await
            .map_err(|e| StackRunnerError::Database(e.to_string()))?;

        // Execute based on run type
        let result = match run.run_type {
            StackRunType::Plan => self.execute_plan(&stack, &run).await,
            StackRunType::Apply => self.execute_apply(&stack, &run).await,
            StackRunType::Destroy => self.execute_destroy(&stack, &run).await,
            StackRunType::Refresh => self.execute_refresh(&stack, &run).await,
        };

        // Update final status
        match result {
            Ok(output) => {
                info!(run_id = %run.id, "Stack run completed successfully");
                self.stack_repo
                    .update_run_finished(run_id, StackRunStatus::Succeeded, None)
                    .await
                    .map_err(|e| StackRunnerError::Database(e.to_string()))?;
                Ok(())
            }
            Err(e) => {
                error!(run_id = %run.id, error = %e, "Stack run failed");
                self.stack_repo
                    .update_run_finished(run_id, StackRunStatus::Failed, Some(&e.to_string()))
                    .await
                    .map_err(|e2| StackRunnerError::Database(e2.to_string()))?;
                Err(e)
            }
        }
    }

    /// Execute a terraform plan.
    async fn execute_plan(
        &self,
        stack: &buildit_core::stack::Stack,
        run: &buildit_core::stack::StackRun,
    ) -> Result<String, StackRunnerError> {
        // Build the container command
        let commands = self.build_plan_commands(stack);

        // Create job spec
        let job_spec = self.build_terraform_job_spec(&format!("plan-{}", run.id), stack, commands);

        // For now, run terraform locally (Phase 2 will add container execution)
        let output = self
            .run_terraform_locally(stack, &["init", "-input=false"])
            .await?;
        let plan_output = self
            .run_terraform_locally(stack, &["plan", "-input=false", "-no-color"])
            .await?;

        let combined_output = format!("{}\n{}", output, plan_output);

        // Parse plan output for resource counts
        let (to_add, to_change, to_destroy) = self.parse_plan_summary(&plan_output);

        // Update run with plan output
        self.stack_repo
            .update_run_plan_output(
                ResourceId::from_uuid(run.id),
                &combined_output,
                None, // TODO: Parse plan JSON
                to_add,
                to_change,
                to_destroy,
            )
            .await
            .map_err(|e| StackRunnerError::Database(e.to_string()))?;

        // If auto-apply is disabled, set status to needs_approval
        if !stack.auto_apply && (to_add > 0 || to_change > 0 || to_destroy > 0) {
            self.stack_repo
                .update_run_status(ResourceId::from_uuid(run.id), StackRunStatus::NeedsApproval)
                .await
                .map_err(|e| StackRunnerError::Database(e.to_string()))?;
        }

        Ok(combined_output)
    }

    /// Execute a terraform apply.
    async fn execute_apply(
        &self,
        stack: &buildit_core::stack::Stack,
        run: &buildit_core::stack::StackRun,
    ) -> Result<String, StackRunnerError> {
        // Init first
        let init_output = self
            .run_terraform_locally(stack, &["init", "-input=false"])
            .await?;

        // Apply
        let apply_output = self
            .run_terraform_locally(
                stack,
                &["apply", "-input=false", "-auto-approve", "-no-color"],
            )
            .await?;

        let combined_output = format!("{}\n{}", init_output, apply_output);

        // Update run with apply output
        self.stack_repo
            .update_run_apply_output(ResourceId::from_uuid(run.id), &combined_output)
            .await
            .map_err(|e| StackRunnerError::Database(e.to_string()))?;

        Ok(combined_output)
    }

    /// Execute a terraform destroy.
    async fn execute_destroy(
        &self,
        stack: &buildit_core::stack::Stack,
        run: &buildit_core::stack::StackRun,
    ) -> Result<String, StackRunnerError> {
        // Init first
        let init_output = self
            .run_terraform_locally(stack, &["init", "-input=false"])
            .await?;

        // Destroy
        let destroy_output = self
            .run_terraform_locally(
                stack,
                &["destroy", "-input=false", "-auto-approve", "-no-color"],
            )
            .await?;

        Ok(format!("{}\n{}", init_output, destroy_output))
    }

    /// Execute a terraform refresh.
    async fn execute_refresh(
        &self,
        stack: &buildit_core::stack::Stack,
        run: &buildit_core::stack::StackRun,
    ) -> Result<String, StackRunnerError> {
        let init_output = self
            .run_terraform_locally(stack, &["init", "-input=false"])
            .await?;
        let refresh_output = self
            .run_terraform_locally(stack, &["refresh", "-no-color"])
            .await?;

        Ok(format!("{}\n{}", init_output, refresh_output))
    }

    /// Build terraform plan commands for container execution.
    fn build_plan_commands(&self, stack: &buildit_core::stack::Stack) -> Vec<String> {
        vec![
            "terraform init -input=false".to_string(),
            format!("terraform plan -input=false -no-color -out=tfplan -detailed-exitcode || true"),
        ]
    }

    /// Build a job spec for running terraform in a container.
    fn build_terraform_job_spec(
        &self,
        _name: &str,
        stack: &buildit_core::stack::Stack,
        commands: Vec<String>,
    ) -> JobSpec {
        let mut env = HashMap::new();

        // Add environment variables from stack config
        if let Some(env_vars) = stack.environment_variables.as_object() {
            for (key, value) in env_vars {
                if let Some(v) = value.as_str() {
                    env.insert(key.clone(), v.to_string());
                }
            }
        }

        // Combine commands into a single shell command
        let command = vec![
            "/bin/sh".to_string(),
            "-c".to_string(),
            commands.join(" && "),
        ];

        JobSpec {
            id: ResourceId::new(),
            image: self.config.terraform_image.clone(),
            command,
            env,
            working_dir: Some(stack.path.clone()),
            timeout: Some(std::time::Duration::from_secs(3600)), // 1 hour timeout
            resources: ResourceRequirements::default(),
            volumes: vec![],
            git_clone: None,
        }
    }

    /// Run terraform command locally (temporary until container execution is ready).
    async fn run_terraform_locally(
        &self,
        stack: &buildit_core::stack::Stack,
        args: &[&str],
    ) -> Result<String, StackRunnerError> {
        use tokio::process::Command;

        let working_dir = stack
            .working_directory
            .as_ref()
            .ok_or_else(|| StackRunnerError::NoWorkingDirectory)?;

        let terraform_bin =
            std::env::var("TERRAFORM_BIN").unwrap_or_else(|_| "terraform".to_string());

        info!(
            working_dir = %working_dir,
            args = ?args,
            "Running terraform command"
        );

        let output = Command::new(&terraform_bin)
            .args(args)
            .current_dir(working_dir)
            .output()
            .await
            .map_err(|e| StackRunnerError::Execution(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}\n{}", stdout, stderr);

        if !output.status.success() {
            // For plan with -detailed-exitcode, exit code 2 means changes detected (not an error)
            if args.contains(&"plan") && output.status.code() == Some(2) {
                return Ok(combined);
            }
            return Err(StackRunnerError::TerraformFailed(combined));
        }

        Ok(combined)
    }

    /// Parse plan output to extract resource change counts.
    fn parse_plan_summary(&self, output: &str) -> (i32, i32, i32) {
        let mut to_add = 0;
        let mut to_change = 0;
        let mut to_destroy = 0;

        // Look for the summary line: "Plan: X to add, Y to change, Z to destroy."
        for line in output.lines() {
            if line.contains("Plan:") && line.contains("to add") {
                // Parse "Plan: 2 to add, 1 to change, 0 to destroy."
                let parts: Vec<&str> = line.split_whitespace().collect();
                for (i, part) in parts.iter().enumerate() {
                    if *part == "to" && i > 0 {
                        if let Ok(n) = parts[i - 1].parse::<i32>() {
                            if i + 1 < parts.len() {
                                match parts[i + 1].trim_end_matches(',') {
                                    "add" => to_add = n,
                                    "change" => to_change = n,
                                    "destroy" | "destroy." => to_destroy = n,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                break;
            }
        }

        (to_add, to_change, to_destroy)
    }
}

/// Errors that can occur during stack execution.
#[derive(Debug, thiserror::Error)]
pub enum StackRunnerError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("No working directory set for stack")]
    NoWorkingDirectory,

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Terraform command failed: {0}")]
    TerraformFailed(String),

    #[error("Job execution error: {0}")]
    JobError(String),
}
