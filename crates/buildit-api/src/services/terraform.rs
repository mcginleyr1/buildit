//! Terraform service for running plan/apply operations.

use buildit_core::stack::{PlanSummary, ResourceChange, StackRunStatus};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Service for Terraform operations.
pub struct TerraformService {
    /// Path to terraform binary
    terraform_bin: String,
}

impl Default for TerraformService {
    fn default() -> Self {
        Self::new()
    }
}

impl TerraformService {
    pub fn new() -> Self {
        let terraform_bin =
            std::env::var("TERRAFORM_BIN").unwrap_or_else(|_| "terraform".to_string());
        Self { terraform_bin }
    }

    /// Initialize a Terraform working directory.
    pub async fn init(
        &self,
        working_dir: &Path,
        backend_config: &HashMap<String, String>,
    ) -> Result<String, TerraformError> {
        info!(dir = %working_dir.display(), "Running terraform init");

        let mut args = vec!["init", "-input=false", "-no-color"];

        // Add backend config options
        let backend_args: Vec<String> = backend_config
            .iter()
            .map(|(k, v)| format!("-backend-config={}={}", k, v))
            .collect();
        for arg in &backend_args {
            args.push(arg);
        }

        let output = Command::new(&self.terraform_bin)
            .args(&args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{}\n{}", stdout, stderr);

        if !output.status.success() {
            error!(output = %combined, "Terraform init failed");
            return Err(TerraformError::InitFailed(combined));
        }

        info!("Terraform init succeeded");
        Ok(combined)
    }

    /// Run terraform plan.
    pub async fn plan(
        &self,
        working_dir: &Path,
        variables: &HashMap<String, String>,
        var_file: Option<&Path>,
        output_tx: Option<mpsc::Sender<String>>,
    ) -> Result<PlanResult, TerraformError> {
        info!(dir = %working_dir.display(), "Running terraform plan");

        let plan_file = working_dir.join("tfplan");

        let mut args = vec![
            "plan".to_string(),
            "-input=false".to_string(),
            "-no-color".to_string(),
            "-detailed-exitcode".to_string(),
            format!("-out={}", plan_file.display()),
        ];

        // Add variables
        for (key, value) in variables {
            args.push(format!("-var={}={}", key, value));
        }

        // Add var file if specified
        if let Some(vf) = var_file {
            args.push(format!("-var-file={}", vf.display()));
        }

        let mut child = Command::new(&self.terraform_bin)
            .args(&args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output_lines = Vec::new();

        // Stream output
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(ref tx) = output_tx {
                                let _ = tx.send(line.clone()).await;
                            }
                            output_lines.push(line);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            warn!(error = %e, "Error reading stdout");
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(ref tx) = output_tx {
                                let _ = tx.send(line.clone()).await;
                            }
                            output_lines.push(line);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            warn!(error = %e, "Error reading stderr");
                        }
                    }
                }
            }
        }

        let status = child.wait().await?;
        let output = output_lines.join("\n");

        // Exit codes:
        // 0 = success, no changes
        // 1 = error
        // 2 = success, changes present
        let has_changes = status.code() == Some(2);
        let success = status.code() == Some(0) || status.code() == Some(2);

        if !success {
            error!(output = %output, "Terraform plan failed");
            return Err(TerraformError::PlanFailed(output));
        }

        // Parse plan output for resource counts
        let summary = self.parse_plan_output(&output);

        info!(
            has_changes = %has_changes,
            to_add = summary.to_add.len(),
            to_change = summary.to_change.len(),
            to_destroy = summary.to_destroy.len(),
            "Terraform plan completed"
        );

        // Get JSON plan for detailed view
        let plan_json = if plan_file.exists() {
            self.show_plan_json(&plan_file).await.ok()
        } else {
            None
        };

        Ok(PlanResult {
            has_changes,
            output,
            plan_file: if has_changes { Some(plan_file) } else { None },
            summary,
            plan_json,
        })
    }

    /// Run terraform apply on a saved plan.
    pub async fn apply(
        &self,
        working_dir: &Path,
        plan_file: &Path,
        output_tx: Option<mpsc::Sender<String>>,
    ) -> Result<ApplyResult, TerraformError> {
        info!(dir = %working_dir.display(), plan = %plan_file.display(), "Running terraform apply");

        let args = vec![
            "apply",
            "-input=false",
            "-no-color",
            "-auto-approve",
            plan_file.to_str().unwrap(),
        ];

        let mut child = Command::new(&self.terraform_bin)
            .args(&args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output_lines = Vec::new();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(ref tx) = output_tx {
                                let _ = tx.send(line.clone()).await;
                            }
                            output_lines.push(line);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            warn!(error = %e, "Error reading stdout");
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(ref tx) = output_tx {
                                let _ = tx.send(line.clone()).await;
                            }
                            output_lines.push(line);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            warn!(error = %e, "Error reading stderr");
                        }
                    }
                }
            }
        }

        let status = child.wait().await?;
        let output = output_lines.join("\n");

        if !status.success() {
            error!(output = %output, "Terraform apply failed");
            return Err(TerraformError::ApplyFailed(output));
        }

        // Get outputs
        let outputs = self.get_outputs(working_dir).await.unwrap_or_default();

        info!(outputs = ?outputs.keys().collect::<Vec<_>>(), "Terraform apply succeeded");

        Ok(ApplyResult { output, outputs })
    }

    /// Run terraform destroy.
    pub async fn destroy(
        &self,
        working_dir: &Path,
        variables: &HashMap<String, String>,
        output_tx: Option<mpsc::Sender<String>>,
    ) -> Result<String, TerraformError> {
        info!(dir = %working_dir.display(), "Running terraform destroy");

        let mut args = vec![
            "destroy".to_string(),
            "-input=false".to_string(),
            "-no-color".to_string(),
            "-auto-approve".to_string(),
        ];

        for (key, value) in variables {
            args.push(format!("-var={}={}", key, value));
        }

        let mut child = Command::new(&self.terraform_bin)
            .args(&args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut output_lines = Vec::new();

        while let Ok(Some(line)) = stdout_reader.next_line().await {
            if let Some(ref tx) = output_tx {
                let _ = tx.send(line.clone()).await;
            }
            output_lines.push(line);
        }

        let status = child.wait().await?;
        let output = output_lines.join("\n");

        if !status.success() {
            return Err(TerraformError::DestroyFailed(output));
        }

        Ok(output)
    }

    /// Get terraform outputs as JSON.
    pub async fn get_outputs(
        &self,
        working_dir: &Path,
    ) -> Result<HashMap<String, serde_json::Value>, TerraformError> {
        let output = Command::new(&self.terraform_bin)
            .args(["output", "-json"])
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Ok(HashMap::new());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let outputs: HashMap<String, TerraformOutput> =
            serde_json::from_str(&json_str).unwrap_or_default();

        // Extract just the values
        Ok(outputs.into_iter().map(|(k, v)| (k, v.value)).collect())
    }

    /// Show plan as JSON.
    async fn show_plan_json(&self, plan_file: &Path) -> Result<serde_json::Value, TerraformError> {
        let output = Command::new(&self.terraform_bin)
            .args(["show", "-json", plan_file.to_str().unwrap()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            return Err(TerraformError::ShowFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&json_str).map_err(|e| TerraformError::ParseFailed(e.to_string()))
    }

    /// Parse plan output to extract resource change counts.
    fn parse_plan_output(&self, output: &str) -> PlanSummary {
        let mut summary = PlanSummary::default();

        // Look for lines like:
        // "Plan: 2 to add, 1 to change, 0 to destroy."
        // Or parse the individual resource lines

        for line in output.lines() {
            let line = line.trim();

            // Parse individual resource changes
            if line.starts_with("# ") {
                // e.g., "# aws_instance.example will be created"
                if let Some(resource) = self.parse_resource_line(line) {
                    match resource.action.as_str() {
                        "create" => summary.to_add.push(resource),
                        "update" => summary.to_change.push(resource),
                        "delete" => summary.to_destroy.push(resource),
                        _ => {}
                    }
                }
            }
        }

        summary
    }

    /// Parse a resource change line from plan output.
    fn parse_resource_line(&self, line: &str) -> Option<ResourceChange> {
        // "# aws_instance.example will be created"
        // "# aws_instance.example will be updated in-place"
        // "# aws_instance.example will be destroyed"

        let line = line.strip_prefix("# ")?;
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 4 {
            return None;
        }

        let address = parts[0].to_string();
        let action = if line.contains("will be created") {
            "create"
        } else if line.contains("will be updated") || line.contains("will be changed") {
            "update"
        } else if line.contains("will be destroyed") {
            "delete"
        } else {
            return None;
        };

        // Parse resource type and name from address
        // e.g., "aws_instance.example" -> type="aws_instance", name="example"
        let (resource_type, name) = if let Some(dot_idx) = address.find('.') {
            (
                address[..dot_idx].to_string(),
                address[dot_idx + 1..].to_string(),
            )
        } else {
            (address.clone(), String::new())
        };

        Some(ResourceChange {
            address,
            resource_type,
            name,
            action: action.to_string(),
            before: None,
            after: None,
        })
    }
}

/// Result of a terraform plan.
#[derive(Debug)]
pub struct PlanResult {
    pub has_changes: bool,
    pub output: String,
    pub plan_file: Option<PathBuf>,
    pub summary: PlanSummary,
    pub plan_json: Option<serde_json::Value>,
}

/// Result of a terraform apply.
#[derive(Debug)]
pub struct ApplyResult {
    pub output: String,
    pub outputs: HashMap<String, serde_json::Value>,
}

/// Terraform output structure.
#[derive(Debug, serde::Deserialize)]
struct TerraformOutput {
    value: serde_json::Value,
    #[allow(dead_code)]
    r#type: serde_json::Value,
    #[allow(dead_code)]
    sensitive: Option<bool>,
}

/// Terraform operation errors.
#[derive(Debug, thiserror::Error)]
pub enum TerraformError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Terraform init failed: {0}")]
    InitFailed(String),

    #[error("Terraform plan failed: {0}")]
    PlanFailed(String),

    #[error("Terraform apply failed: {0}")]
    ApplyFailed(String),

    #[error("Terraform destroy failed: {0}")]
    DestroyFailed(String),

    #[error("Terraform show failed: {0}")]
    ShowFailed(String),

    #[error("Failed to parse terraform output: {0}")]
    ParseFailed(String),
}
