//! Pipeline management endpoints.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use buildit_config::VariableContextBuilder;
use buildit_core::ResourceId;
use buildit_core::pipeline::Pipeline;
use buildit_db::{LogRepo, PipelineRepo};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_pipelines).post(create_pipeline))
        .route("/{id}", get(get_pipeline))
        .route("/{id}/runs", get(list_runs).post(trigger_run))
        .route("/{id}/runs/{run_id}/logs", get(get_run_logs))
}

#[derive(Debug, Deserialize)]
struct ListPipelinesQuery {
    tenant_id: Uuid,
}

#[derive(Debug, Serialize)]
struct PipelineResponse {
    id: String,
    name: String,
    repository: String,
}

async fn list_pipelines(
    State(state): State<AppState>,
    Query(query): Query<ListPipelinesQuery>,
) -> Result<Json<Vec<PipelineResponse>>, ApiError> {
    let tenant_id = ResourceId::from_uuid(query.tenant_id);
    let pipelines = state.pipeline_repo.list_by_tenant(tenant_id).await?;
    let response: Vec<PipelineResponse> = pipelines
        .into_iter()
        .map(|p| PipelineResponse {
            id: p.id.to_string(),
            name: p.name,
            repository: p.repository,
        })
        .collect();
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreatePipelineRequest {
    tenant_id: Uuid,
    name: String,
    repository: String,
    config: serde_json::Value,
}

async fn create_pipeline(
    State(state): State<AppState>,
    Json(req): Json<CreatePipelineRequest>,
) -> Result<Json<PipelineResponse>, ApiError> {
    let tenant_id = ResourceId::from_uuid(req.tenant_id);
    let pipeline = state
        .pipeline_repo
        .create(tenant_id, &req.name, &req.repository, req.config.clone())
        .await?;

    // Extract and create stage definitions from config
    let pipeline_id = ResourceId::from_uuid(pipeline.id);
    if let Some(stages) = req.config.get("stages").and_then(|s| s.as_array()) {
        for stage in stages {
            let name = stage
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unnamed");
            let image = stage.get("image").and_then(|i| i.as_str());
            let commands: Vec<String> = stage
                .get("commands")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let depends_on: Vec<String> = stage
                .get("depends_on")
                .and_then(|d| d.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let env = stage.get("env").cloned().unwrap_or(serde_json::json!({}));
            let timeout = stage
                .get("timeout_seconds")
                .and_then(|t| t.as_i64())
                .map(|t| t as i32);

            if let Err(e) = state
                .pipeline_repo
                .create_stage(
                    pipeline_id,
                    name,
                    image,
                    &commands,
                    &depends_on,
                    env,
                    timeout,
                )
                .await
            {
                tracing::error!(error = %e, stage = %name, "Failed to create stage definition");
            }
        }
    }

    Ok(Json(PipelineResponse {
        id: pipeline.id.to_string(),
        name: pipeline.name,
        repository: pipeline.repository,
    }))
}

async fn get_pipeline(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PipelineResponse>, ApiError> {
    let pipeline = state
        .pipeline_repo
        .get_by_id(ResourceId::from_uuid(id))
        .await?;
    Ok(Json(PipelineResponse {
        id: pipeline.id.to_string(),
        name: pipeline.name,
        repository: pipeline.repository,
    }))
}

#[derive(Debug, Serialize)]
struct RunResponse {
    id: String,
    number: i64,
    status: String,
}

async fn list_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<RunResponse>>, ApiError> {
    let runs = state
        .pipeline_repo
        .list_runs(ResourceId::from_uuid(id), 20)
        .await?;
    let response: Vec<RunResponse> = runs
        .into_iter()
        .map(|r| RunResponse {
            id: r.id.to_string(),
            number: r.number,
            status: r.status,
        })
        .collect();
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct TriggerRunRequest {
    branch: Option<String>,
    sha: Option<String>,
}

async fn trigger_run(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TriggerRunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    let trigger_info = serde_json::json!({
        "kind": "manual"
    });
    let git_info = serde_json::json!({
        "branch": req.branch.clone().unwrap_or_default(),
        "sha": req.sha.clone().unwrap_or_default(),
        "short_sha": "",
        "message": "",
        "author": ""
    });

    // Create the run record
    let run = state
        .pipeline_repo
        .create_run(ResourceId::from_uuid(id), trigger_info, git_info)
        .await?;

    // Get the pipeline config
    let pipeline_record = state
        .pipeline_repo
        .get_by_id(ResourceId::from_uuid(id))
        .await?;

    // Parse config - extract stages and env from stored JSON
    let config = &pipeline_record.config;
    let stages: Vec<buildit_core::pipeline::Stage> =
        serde_json::from_value(config.get("stages").cloned().unwrap_or_default())
            .map_err(|e| ApiError::Internal(format!("Invalid stages config: {}", e)))?;
    let env: HashMap<String, String> =
        serde_json::from_value(config.get("env").cloned().unwrap_or_default()).unwrap_or_default();
    let triggers: Vec<buildit_core::pipeline::Trigger> =
        serde_json::from_value(config.get("triggers").cloned().unwrap_or_default())
            .unwrap_or_default();

    // Build Pipeline struct
    let pipeline = Pipeline {
        id: ResourceId::from_uuid(pipeline_record.id),
        name: pipeline_record.name.clone(),
        tenant_id: ResourceId::from_uuid(pipeline_record.tenant_id),
        repository: pipeline_record.repository.clone(),
        triggers,
        stages,
        env,
        caches: vec![],
    };

    // Execute pipeline in background (if orchestrator is available)
    let orchestrator = state.orchestrator.clone();
    let pipeline_repo = state.pipeline_repo.clone();
    let log_repo = state.log_repo.clone();
    let broadcaster = state.broadcaster.clone();
    let run_id = ResourceId::from_uuid(run.id);
    let run_id_str = run.id.to_string();

    if let Some(orchestrator) = orchestrator {
        let stage_names: Vec<String> = pipeline.stages.iter().map(|s| s.name.clone()).collect();

        tokio::spawn(async move {
            tracing::info!(run_id = %run_id, "Starting pipeline execution");

            // Create stage result records for all stages upfront
            for stage_name in &stage_names {
                if let Err(e) = pipeline_repo.create_stage_result(run_id, stage_name).await {
                    tracing::error!(error = %e, stage = %stage_name, "Failed to create stage result");
                }
            }

            // Set run status to running
            if let Err(e) = pipeline_repo.update_run_status(run_id, "running").await {
                tracing::error!(error = %e, "Failed to update run status to running");
                return;
            }

            // Build environment
            let mut env = HashMap::new();
            env.insert("CI".to_string(), "true".to_string());
            env.insert("BUILDIT".to_string(), "true".to_string());

            // Build variable context for interpolation
            // Extract git info from JSON
            let git_branch = run
                .git_info
                .get("branch")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let git_sha = run
                .git_info
                .get("sha")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();

            let var_ctx = VariableContextBuilder::new()
                .with_pipeline(pipeline.id.to_string(), pipeline.name.clone())
                .with_run(run_id.to_string(), run.number as u32)
                .with_git_branch(git_branch)
                .with_git_sha(git_sha)
                .build();

            // Execute
            tracing::info!(run_id = %run_id, "Executing pipeline with {} stages", pipeline.stages.len());
            let (event_rx, result_handle) = orchestrator.execute(&pipeline, env, Some(var_ctx));

            // Process events and update stage results in database
            let mut event_rx = event_rx;
            let repo_clone = pipeline_repo.clone();
            let log_repo_clone = log_repo.clone();
            while let Some(event) = event_rx.recv().await {
                match event {
                    buildit_scheduler::PipelineEvent::StageStarted { stage } => {
                        tracing::info!(run_id = %run_id, stage = %stage, "Stage started");
                        if let Err(e) = repo_clone
                            .update_stage_result_started(run_id, &stage, None)
                            .await
                        {
                            tracing::error!(error = %e, "Failed to update stage start");
                        }
                    }
                    buildit_scheduler::PipelineEvent::StageCompleted { stage, success } => {
                        let status = if success { "succeeded" } else { "failed" };
                        let error_msg = if success { None } else { Some("Stage failed") };
                        tracing::info!(run_id = %run_id, stage = %stage, status = %status, "Stage completed");
                        if let Err(e) = repo_clone
                            .update_stage_result_finished(run_id, &stage, status, error_msg)
                            .await
                        {
                            tracing::error!(error = %e, "Failed to update stage finish");
                        }
                    }
                    buildit_scheduler::PipelineEvent::StageLog { stage, line } => {
                        // Store log line to database
                        let stream = match line.stream {
                            buildit_core::executor::LogStream::Stdout => "stdout",
                            buildit_core::executor::LogStream::Stderr => "stderr",
                            buildit_core::executor::LogStream::System => "system",
                        };
                        if let Err(e) = log_repo_clone
                            .append_log(run_id, &stage, stream, &line.content)
                            .await
                        {
                            tracing::error!(error = %e, "Failed to store log line");
                        }
                    }
                    buildit_scheduler::PipelineEvent::PipelineCompleted { success } => {
                        tracing::info!(run_id = %run_id, success = %success, "Pipeline completed");
                    }
                }
            }

            let result = result_handle.await.expect("Pipeline execution task failed");

            // Update final status
            let status = if result.success {
                tracing::info!(run_id = %run_id, "Pipeline succeeded");
                "succeeded"
            } else {
                tracing::warn!(run_id = %run_id, "Pipeline failed");
                "failed"
            };
            if let Err(e) = pipeline_repo.update_run_status(run_id, status).await {
                tracing::error!(error = %e, "Failed to update run status to {}", status);
            }
        });
    } else {
        tracing::warn!(run_id = %run_id, "Orchestrator unavailable - run created but not executed");
    }

    Ok(Json(RunResponse {
        id: run.id.to_string(),
        number: run.number,
        status: "pending".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct GetLogsQuery {
    stage: Option<String>,
    offset: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct LogEntry {
    id: String,
    stage_name: String,
    timestamp: String,
    stream: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct LogsResponse {
    logs: Vec<LogEntry>,
    has_more: bool,
}

async fn get_run_logs(
    State(state): State<AppState>,
    Path((_pipeline_id, run_id)): Path<(Uuid, Uuid)>,
    Query(query): Query<GetLogsQuery>,
) -> Result<Json<LogsResponse>, ApiError> {
    let run_id = ResourceId::from_uuid(run_id);
    let offset = query.offset.unwrap_or(0);
    let limit = query.limit.unwrap_or(500).min(1000); // Cap at 1000 lines

    let logs = state
        .log_repo
        .get_logs_paginated(run_id, query.stage.as_deref(), offset, limit + 1)
        .await?;

    // Check if there are more logs
    let has_more = logs.len() > limit as usize;
    let logs: Vec<LogEntry> = logs
        .into_iter()
        .take(limit as usize)
        .map(|log| LogEntry {
            id: log.id.to_string(),
            stage_name: log.stage_name,
            timestamp: log.timestamp.to_rfc3339(),
            stream: log.stream,
            content: log.content,
        })
        .collect();

    Ok(Json(LogsResponse { logs, has_more }))
}
