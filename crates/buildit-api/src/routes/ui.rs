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

// ============================================================================
// Template structs
// ============================================================================

#[derive(Template)]
#[template(path = "pages/dashboard.html")]
struct DashboardTemplate {
    pipeline_count: i64,
    run_count_today: i64,
    success_rate: String,
    recent_runs: Vec<RecentRunView>,
}

#[derive(Template)]
#[template(path = "pages/pipelines/list.html")]
struct PipelinesTemplate {
    tenant_id: String,
    pipelines: Vec<PipelineView>,
}

#[derive(Template)]
#[template(path = "pages/pipelines/detail.html")]
struct PipelineDetailTemplate {
    pipeline: PipelineView,
    runs: Vec<RunView>,
    has_runs: bool,
}

#[derive(Template)]
#[template(path = "pages/pipelines/run.html")]
struct RunDetailTemplate {
    pipeline: PipelineView,
    run: RunView,
    stages: Vec<StageView>,
    first_stage_name: String,
}

#[derive(Template)]
#[template(path = "pages/environments/list.html")]
struct EnvironmentsTemplate {
    environments: Vec<EnvironmentView>,
}

#[derive(Template)]
#[template(path = "pages/settings/index.html")]
struct SettingsTemplate {
    tenant_name: String,
    tenant_slug: String,
}

// ============================================================================
// View models
// ============================================================================

struct PipelineView {
    id: String,
    name: String,
    repository: String,
    last_run_id: String,
    last_run_number: i64,
    last_run_status: String,
    last_run_ago: String,
}

struct RunView {
    id: String,
    number: i64,
    status: String,
    branch: String,
    commit_sha: String,
    commit_message: String,
    trigger_kind: String,
    created_at: String,
    duration: String,
}

struct RecentRunView {
    pipeline_id: String,
    pipeline_name: String,
    run_id: String,
    run_number: i64,
    status: String,
    ago: String,
}

struct StageView {
    name: String,
    status: String,
    duration: String,
    dependencies: Vec<String>,
}

struct EnvironmentView {
    name: String,
    service_count: i32,
    health_status: String,
    target_name: String,
    target_type: String,
    last_deploy_ago: String,
}

// ============================================================================
// Routes
// ============================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        // Dashboard
        .route("/", get(dashboard_page))
        // Pipelines
        .route("/pipelines", get(pipelines_page))
        .route("/pipelines/{id}", get(pipeline_detail_page))
        .route("/pipelines/{id}/runs/{run_id}", get(run_detail_page))
        // Runs (alias)
        .route("/runs", get(runs_page))
        // Deployments
        .route("/environments", get(environments_page))
        .route("/services", get(services_page))
        .route("/history", get(history_page))
        // Infrastructure
        .route("/targets", get(targets_page))
        // Settings
        .route("/settings", get(settings_page))
}

// ============================================================================
// Page handlers
// ============================================================================

async fn dashboard_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Get default tenant
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let pipelines = state.pipeline_repo.list_by_tenant(tenant_id).await?;
    let pipeline_count = pipelines.len() as i64;

    // Gather recent runs from all pipelines
    let mut recent_runs = Vec::new();
    let mut total_runs = 0i64;
    let mut successful_runs = 0i64;

    for p in &pipelines {
        let runs = state
            .pipeline_repo
            .list_runs(ResourceId::from_uuid(p.id), 5)
            .await
            .unwrap_or_default();

        for run in runs {
            total_runs += 1;
            if run.status == "succeeded" {
                successful_runs += 1;
            }

            if recent_runs.len() < 10 {
                recent_runs.push(RecentRunView {
                    pipeline_id: p.id.to_string(),
                    pipeline_name: p.name.clone(),
                    run_id: run.id.to_string(),
                    run_number: run.number,
                    status: run.status,
                    ago: format_time_ago(run.created_at),
                });
            }
        }
    }

    let success_rate = if total_runs > 0 {
        format!(
            "{:.1}",
            (successful_runs as f64 / total_runs as f64) * 100.0
        )
    } else {
        "0.0".to_string()
    };

    let template = DashboardTemplate {
        pipeline_count,
        run_count_today: total_runs,
        success_rate,
        recent_runs,
    };

    Ok(Html(template.render().unwrap()))
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

        let (last_run_id, last_run_number, last_run_status, last_run_ago) =
            if let Some(run) = runs.first() {
                (
                    run.id.to_string(),
                    run.number,
                    run.status.clone(),
                    format_time_ago(run.created_at),
                )
            } else {
                (String::new(), 0, String::new(), String::new())
            };

        pipelines.push(PipelineView {
            id: p.id.to_string(),
            name: p.name,
            repository: p.repository,
            last_run_id,
            last_run_number,
            last_run_status,
            last_run_ago,
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
        .map(|r| {
            let branch = r
                .trigger_info
                .get("branch")
                .and_then(|v| v.as_str())
                .unwrap_or("main")
                .to_string();
            let commit_sha = r
                .trigger_info
                .get("commit_sha")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .chars()
                .take(7)
                .collect();
            let commit_message = r
                .trigger_info
                .get("commit_message")
                .and_then(|v| v.as_str())
                .unwrap_or("No message")
                .to_string();

            RunView {
                id: r.id.to_string(),
                number: r.number,
                status: r.status,
                branch,
                commit_sha,
                commit_message,
                trigger_kind: r
                    .trigger_info
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("manual")
                    .to_string(),
                created_at: format_time_ago(r.created_at),
                duration: "1m 23s".to_string(), // TODO: Calculate actual duration
            }
        })
        .collect();

    let has_runs = !runs.is_empty();
    let template = PipelineDetailTemplate {
        pipeline: PipelineView {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            repository: pipeline.repository,
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
            last_run_ago: String::new(),
        },
        runs,
        has_runs,
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

    let branch = run
        .trigger_info
        .get("branch")
        .and_then(|v| v.as_str())
        .unwrap_or("main")
        .to_string();
    let commit_sha = run
        .trigger_info
        .get("commit_sha")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(7)
        .collect();
    let commit_message = run
        .trigger_info
        .get("commit_message")
        .and_then(|v| v.as_str())
        .unwrap_or("No message")
        .to_string();

    // TODO: Get actual stages from database
    // For now, return placeholder stages based on typical pipeline
    let stages = vec![
        StageView {
            name: "checkout".to_string(),
            status: "succeeded".to_string(),
            duration: "2s".to_string(),
            dependencies: vec![],
        },
        StageView {
            name: "test".to_string(),
            status: "succeeded".to_string(),
            duration: "45s".to_string(),
            dependencies: vec!["checkout".to_string()],
        },
        StageView {
            name: "build".to_string(),
            status: "succeeded".to_string(),
            duration: "1m 12s".to_string(),
            dependencies: vec!["test".to_string()],
        },
        StageView {
            name: "deploy".to_string(),
            status: if run.status == "running" {
                "running"
            } else {
                "succeeded"
            }
            .to_string(),
            duration: "15s".to_string(),
            dependencies: vec!["build".to_string()],
        },
    ];

    let first_stage_name = stages.first().map(|s| s.name.clone()).unwrap_or_default();
    let template = RunDetailTemplate {
        pipeline: PipelineView {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            repository: pipeline.repository,
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
            last_run_ago: String::new(),
        },
        run: RunView {
            id: run.id.to_string(),
            number: run.number,
            status: run.status,
            branch,
            commit_sha,
            commit_message,
            trigger_kind: run
                .trigger_info
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("manual")
                .to_string(),
            created_at: format_time_ago(run.created_at),
            duration: "2m 14s".to_string(), // TODO: Calculate actual duration
        },
        stages,
        first_stage_name,
    };

    Ok(Html(template.render().unwrap()))
}

async fn runs_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Redirect to pipelines for now, will implement dedicated runs page later
    pipelines_page(State(state)).await
}

async fn environments_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Placeholder environments
    let environments = vec![
        EnvironmentView {
            name: "development".to_string(),
            service_count: 4,
            health_status: "healthy".to_string(),
            target_name: "dev-cluster".to_string(),
            target_type: "kubernetes".to_string(),
            last_deploy_ago: "5 minutes ago".to_string(),
        },
        EnvironmentView {
            name: "staging".to_string(),
            service_count: 4,
            health_status: "healthy".to_string(),
            target_name: "stg-cluster".to_string(),
            target_type: "kubernetes".to_string(),
            last_deploy_ago: "2 hours ago".to_string(),
        },
        EnvironmentView {
            name: "production".to_string(),
            service_count: 4,
            health_status: "degraded".to_string(),
            target_name: "prd-cluster".to_string(),
            target_type: "kubernetes".to_string(),
            last_deploy_ago: "1 day ago".to_string(),
        },
    ];

    let template = EnvironmentsTemplate { environments };
    Ok(Html(template.render().unwrap()))
}

async fn services_page(State(_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Redirect to environments for now
    Ok(Html(
        "<script>window.location.href = '/environments';</script>".to_string(),
    ))
}

async fn history_page(State(_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Redirect to environments for now
    Ok(Html(
        "<script>window.location.href = '/environments';</script>".to_string(),
    ))
}

async fn targets_page(State(_state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Redirect to environments for now
    Ok(Html(
        "<script>window.location.href = '/environments';</script>".to_string(),
    ))
}

async fn settings_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let template = SettingsTemplate {
        tenant_name: tenant.name,
        tenant_slug: tenant.slug,
    };

    Ok(Html(template.render().unwrap()))
}

// ============================================================================
// Helpers
// ============================================================================

fn format_time_ago(time: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(time);

    if duration.num_seconds() < 60 {
        format!("{}s ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}
