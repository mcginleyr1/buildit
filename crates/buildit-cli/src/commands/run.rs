//! Local pipeline execution command.

use anyhow::{Context, Result};
use buildit_config::pipeline::parse_pipeline;
use buildit_executor::LocalDockerExecutor;
use buildit_scheduler::{PipelineEvent, PipelineOrchestrator};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Run a pipeline locally using Docker.
pub async fn run_local(config_path: &str, stages: Option<Vec<String>>) -> Result<()> {
    // Read and parse the pipeline config
    let content = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path))?;

    let pipeline = parse_pipeline(&content)
        .with_context(|| format!("Failed to parse pipeline config: {}", config_path))?;

    println!("Running pipeline: {}", pipeline.name);
    println!("Stages: {}", pipeline.stages.len());

    // Filter stages if specified
    let pipeline = if let Some(stage_filter) = stages {
        let mut filtered = pipeline.clone();
        filtered.stages.retain(|s| stage_filter.contains(&s.name));
        if filtered.stages.is_empty() {
            anyhow::bail!("No matching stages found for filter: {:?}", stage_filter);
        }
        println!("Running filtered stages: {:?}", stage_filter);
        filtered
    } else {
        pipeline
    };

    // Create the Docker executor
    let executor = LocalDockerExecutor::new().context("Failed to connect to Docker")?;
    let executor = Arc::new(executor);

    // Create the orchestrator
    let orchestrator = PipelineOrchestrator::new(executor);

    // Build environment variables
    let mut env = HashMap::new();
    env.insert("CI".to_string(), "true".to_string());
    env.insert("BUILDIT".to_string(), "true".to_string());

    // Execute the pipeline
    println!("\n--- Starting pipeline execution ---\n");

    let (mut rx, result) = orchestrator.execute(&pipeline, env).await;

    // Process events
    while let Some(event) = rx.recv().await {
        match event {
            PipelineEvent::StageStarted { stage } => {
                println!("▶ Stage '{}' started", stage);
            }
            PipelineEvent::StageLog { stage, line } => {
                let stream_marker = match line.stream {
                    buildit_core::executor::LogStream::Stdout => " ",
                    buildit_core::executor::LogStream::Stderr => "!",
                    buildit_core::executor::LogStream::System => "*",
                };
                println!("  [{}]{} {}", stage, stream_marker, line.content);
            }
            PipelineEvent::StageCompleted { stage, success } => {
                if success {
                    println!("✓ Stage '{}' completed successfully\n", stage);
                } else {
                    println!("✗ Stage '{}' failed\n", stage);
                }
            }
            PipelineEvent::PipelineCompleted { success } => {
                if success {
                    println!("--- Pipeline completed successfully ---");
                } else {
                    println!("--- Pipeline failed ---");
                }
            }
        }
    }

    // Print summary
    println!("\n--- Stage Summary ---");
    for (stage_name, state) in &result.stage_states {
        let status = match state {
            buildit_scheduler::StageState::Succeeded => "✓ succeeded",
            buildit_scheduler::StageState::Failed { message } => &format!("✗ failed: {}", message),
            buildit_scheduler::StageState::Skipped { reason } => &format!("⊘ skipped: {}", reason),
            buildit_scheduler::StageState::Pending => "○ pending",
            buildit_scheduler::StageState::Running { .. } => "▶ running",
        };
        println!("  {} - {}", stage_name, status);
    }

    if result.success {
        println!("\n✓ Pipeline succeeded!");
        Ok(())
    } else {
        anyhow::bail!("Pipeline failed");
    }
}
