//! Pipeline management endpoints.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use buildit_core::ResourceId;
use buildit_db::PipelineRepo;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_pipelines).post(create_pipeline))
        .route("/{id}", get(get_pipeline))
        .route("/{id}/runs", get(list_runs).post(trigger_run))
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
        .create(tenant_id, &req.name, &req.repository, req.config)
        .await?;
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
        "branch": req.branch,
        "sha": req.sha.unwrap_or_default(),
        "short_sha": "",
        "message": "",
        "author": ""
    });

    let run = state
        .pipeline_repo
        .create_run(ResourceId::from_uuid(id), trigger_info, git_info)
        .await?;

    Ok(Json(RunResponse {
        id: run.id.to_string(),
        number: run.number,
        status: run.status,
    }))
}
