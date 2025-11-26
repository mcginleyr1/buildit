//! UI routes serving HTML templates.

use askama::Template;
use axum::Router;
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use buildit_core::ResourceId;
use buildit_db::{PipelineRepo, TenantRepo};

// Template structs

#[derive(Template)]
#[template(path = "pipelines.html")]
struct PipelinesTemplate {
    tenant_id: String,
    pipelines: Vec<PipelineView>,
}

#[derive(Template)]
#[template(path = "pipeline_detail.html")]
struct PipelineDetailTemplate {
    pipeline: PipelineView,
    runs: Vec<RunView>,
}

#[derive(Template)]
#[template(path = "run_detail.html")]
struct RunDetailTemplate {
    pipeline: PipelineView,
    run: RunView,
    stages: Vec<StageView>,
}

// View models

struct PipelineView {
    id: String,
    name: String,
    repository: String,
    last_run_id: String,
    last_run_number: i64,
    last_run_status: String,
}

struct RunView {
    id: String,
    number: i64,
    status: String,
    trigger_kind: String,
    created_at: String,
}

struct StageView {
    name: String,
    status: String,
}

// Routes

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(pipelines_page))
        .route("/pipelines/{id}", get(pipeline_detail_page))
        .route("/pipelines/{id}/runs/{run_id}", get(run_detail_page))
}

async fn pipelines_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Get default tenant
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let pipeline_records = state.pipeline_repo.list_by_tenant(tenant_id).await?;

    let mut pipelines = Vec::new();
    for p in pipeline_records {
        let runs = state
            .pipeline_repo
            .list_runs(ResourceId::from_uuid(p.id), 1)
            .await
            .unwrap_or_default();

        let (last_run_id, last_run_number, last_run_status) = if let Some(run) = runs.first() {
            (run.id.to_string(), run.number, run.status.clone())
        } else {
            (String::new(), 0, String::new())
        };

        pipelines.push(PipelineView {
            id: p.id.to_string(),
            name: p.name,
            repository: p.repository,
            last_run_id,
            last_run_number,
            last_run_status,
        });
    }

    let template = PipelinesTemplate {
        tenant_id: tenant.id.to_string(),
        pipelines,
    };

    Ok(Html(template.render().unwrap()))
}

async fn pipeline_detail_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let pipeline = state
        .pipeline_repo
        .get_by_id(ResourceId::from_uuid(id))
        .await?;

    let run_records = state
        .pipeline_repo
        .list_runs(ResourceId::from_uuid(id), 20)
        .await?;

    let runs: Vec<RunView> = run_records
        .into_iter()
        .map(|r| RunView {
            id: r.id.to_string(),
            number: r.number,
            status: r.status,
            trigger_kind: r
                .trigger_info
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            created_at: r.created_at.format("%Y-%m-%d %H:%M").to_string(),
        })
        .collect();

    let template = PipelineDetailTemplate {
        pipeline: PipelineView {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            repository: pipeline.repository,
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
        },
        runs,
    };

    Ok(Html(template.render().unwrap()))
}

async fn run_detail_page(
    State(state): State<AppState>,
    Path((pipeline_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let pipeline = state
        .pipeline_repo
        .get_by_id(ResourceId::from_uuid(pipeline_id))
        .await?;

    let run = state
        .pipeline_repo
        .get_run(ResourceId::from_uuid(run_id))
        .await?;

    // TODO: Get actual stages from database
    // For now, return placeholder stages
    let stages = vec![
        StageView {
            name: "build".to_string(),
            status: "succeeded".to_string(),
        },
        StageView {
            name: "test".to_string(),
            status: "succeeded".to_string(),
        },
        StageView {
            name: "deploy".to_string(),
            status: "pending".to_string(),
        },
    ];

    let template = RunDetailTemplate {
        pipeline: PipelineView {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            repository: pipeline.repository,
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
        },
        run: RunView {
            id: run.id.to_string(),
            number: run.number,
            status: run.status,
            trigger_kind: run
                .trigger_info
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            created_at: run.created_at.format("%Y-%m-%d %H:%M").to_string(),
        },
        stages,
    };

    Ok(Html(template.render().unwrap()))
}
