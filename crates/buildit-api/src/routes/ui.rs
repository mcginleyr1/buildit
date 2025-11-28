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
use buildit_db::{DeploymentRepo, PipelineRepo, TenantRepo};

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
    has_recent_runs: bool,
}

#[derive(Template)]
#[template(path = "pages/pipelines/list.html")]
struct PipelinesTemplate {
    tenant_id: String,
    pipelines: Vec<PipelineView>,
    has_pipelines: bool,
}

#[derive(Template)]
#[template(path = "pages/pipelines/new.html")]
struct NewPipelineTemplate {
    tenant_id: String,
    pipeline_name_default: String,
    available_secrets: Vec<SecretView>,
    available_targets: Vec<TargetView>,
    available_environments: Vec<EnvironmentSelectView>,
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
    edges: Vec<DagEdge>,
    first_stage_name: String,
    dag_width: i32,
    dag_height: i32,
}

#[derive(Template)]
#[template(path = "pages/environments/list.html")]
struct EnvironmentsTemplate {
    environments: Vec<EnvironmentView>,
    has_environments: bool,
}

#[derive(Template)]
#[template(path = "pages/settings/index.html")]
struct SettingsTemplate {
    tenant_name: String,
    tenant_slug: String,
}

#[derive(Template)]
#[template(path = "pages/settings/team.html")]
struct SettingsTeamTemplate {
    members: Vec<TeamMemberView>,
}

#[derive(Template)]
#[template(path = "pages/settings/secrets.html")]
struct SettingsSecretsTemplate {
    secrets: Vec<SecretView>,
}

#[derive(Template)]
#[template(path = "pages/settings/tokens.html")]
struct SettingsTokensTemplate {
    tokens: Vec<TokenView>,
}

#[derive(Template)]
#[template(path = "pages/settings/git.html")]
struct SettingsGitTemplate {
    org_id: String,
    github_connected: bool,
    github_username: String,
    gitlab_connected: bool,
    gitlab_username: String,
    bitbucket_connected: bool,
    bitbucket_username: String,
}

#[derive(Template)]
#[template(path = "pages/settings/notifications.html")]
struct SettingsNotificationsTemplate {
    slack_connected: bool,
    slack_channel: String,
    has_webhooks: bool,
    webhook_count: i32,
}

#[derive(Template)]
#[template(path = "pages/deployments/services.html")]
struct ServicesTemplate {
    services: Vec<ServiceView>,
    has_services: bool,
}

#[derive(Template)]
#[template(path = "pages/deployments/history.html")]
struct HistoryTemplate {
    deployments: Vec<DeploymentView>,
    has_deployments: bool,
}

#[derive(Template)]
#[template(path = "pages/infrastructure/targets.html")]
struct TargetsTemplate {
    targets: Vec<TargetView>,
    has_targets: bool,
}

#[derive(Template)]
#[template(path = "pages/runs/list.html")]
struct RunsTemplate {
    pipelines: Vec<PipelineView>,
    runs: Vec<AllRunView>,
    has_runs: bool,
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
    #[allow(dead_code)]
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
    /// Column/group this stage belongs to (computed from dependencies)
    column: i32,
    /// Row within the column (for parallel stages)
    row: i32,
    // DAG layout computed fields (legacy, kept for compatibility)
    x: i32,
    y: i32,
}

/// A column in the pipeline visualization (e.g., BUILD, TEST, DEPLOY)
struct PipelineColumn {
    /// Column index (0-based)
    index: i32,
    /// Display name for the column header
    name: String,
    /// Stages in this column
    stages: Vec<String>,
}

/// Edge between two stages for DAG visualization
struct DagEdge {
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    /// Status of the source stage (for edge coloring)
    from_status: String,
    /// Name of source stage
    from_name: String,
    /// Name of target stage
    to_name: String,
    /// Control point offset for bezier curves (helps with edge routing)
    control_offset: i32,
}

struct EnvironmentView {
    name: String,
    service_count: i32,
    health_status: String,
    target_name: String,
    target_type: String,
    last_deploy_ago: String,
}

struct ServiceView {
    name: String,
    image: String,
    status: String,
    environments: Vec<String>,
    last_deploy_ago: String,
}

struct DeploymentView {
    version: String,
    commit_sha: String,
    service_name: String,
    environment: String,
    status: String,
    deployed_ago: String,
    duration: String,
}

struct TargetView {
    name: String,
    target_type: String,
    status: String,
    region: String,
    environment_count: i32,
}

struct AllRunView {
    id: String,
    pipeline_id: String,
    pipeline_name: String,
    number: i64,
    status: String,
    branch: String,
    commit_sha: String,
    commit_message: String,
    created_at: String,
    duration: String,
}

struct TeamMemberView {
    name: String,
    email: String,
    role: String,
    initials: String,
}

struct SecretView {
    name: String,
    updated_at: String,
}

struct TokenView {
    name: String,
    prefix: String,
    last_used: String,
    expires: String,
}

struct EnvironmentSelectView {
    name: String,
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
        .route("/pipelines/new", get(new_pipeline_page))
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
        .route("/settings/team", get(settings_team_page))
        .route("/settings/secrets", get(settings_secrets_page))
        .route("/settings/tokens", get(settings_tokens_page))
        .route("/settings/git", get(settings_git_page))
        .route("/settings/notifications", get(settings_notifications_page))
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

    let has_recent_runs = !recent_runs.is_empty();
    let template = DashboardTemplate {
        pipeline_count,
        run_count_today: total_runs,
        success_rate,
        recent_runs,
        has_recent_runs,
    };

    match template.render() {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            tracing::error!("Dashboard template render error: {}", e);
            Err(ApiError::Internal(format!("Template error: {}", e)))
        }
    }
}

async fn new_pipeline_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);

    // Get available targets for deployment step
    let target_records = state.deployment_repo.list_targets(tenant_id).await?;
    let available_targets: Vec<TargetView> = target_records
        .into_iter()
        .map(|t| TargetView {
            name: t.name,
            target_type: t.target_type,
            status: t.status,
            region: t.region.unwrap_or_else(|| "-".to_string()),
            environment_count: 0,
        })
        .collect();

    // Get available environments for deployment mapping
    let env_records = state.deployment_repo.list_environments(tenant_id).await?;
    let available_environments: Vec<EnvironmentSelectView> = env_records
        .into_iter()
        .map(|e| EnvironmentSelectView { name: e.name })
        .collect();

    // Placeholder secrets (TODO: load from secrets table when available)
    let available_secrets = vec![
        SecretView {
            name: "DOCKER_PASSWORD".to_string(),
            updated_at: String::new(),
        },
        SecretView {
            name: "AWS_ACCESS_KEY_ID".to_string(),
            updated_at: String::new(),
        },
        SecretView {
            name: "AWS_SECRET_ACCESS_KEY".to_string(),
            updated_at: String::new(),
        },
    ];

    let template = NewPipelineTemplate {
        tenant_id: tenant.id.to_string(),
        pipeline_name_default: "my-app".to_string(),
        available_secrets,
        available_targets,
        available_environments,
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

    let has_pipelines = !pipelines.is_empty();
    let template = PipelinesTemplate {
        tenant_id: tenant.id.to_string(),
        pipelines,
        has_pipelines,
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

    // Load stages from database
    let stage_definitions = state
        .pipeline_repo
        .list_stages(ResourceId::from_uuid(pipeline_id))
        .await?;

    let stage_results = state
        .pipeline_repo
        .list_stage_results(ResourceId::from_uuid(run_id))
        .await?;

    // Calculate total run duration from stage results
    let run_duration = {
        let earliest_start = stage_results.iter().filter_map(|r| r.started_at).min();
        let latest_end = stage_results.iter().filter_map(|r| r.finished_at).max();
        match (earliest_start, latest_end) {
            (Some(start), Some(end)) => {
                let secs = (end - start).num_seconds();
                if secs < 60 {
                    format!("{}s", secs)
                } else {
                    format!("{}m {}s", secs / 60, secs % 60)
                }
            }
            (Some(start), None) => {
                // Still running
                let secs = (chrono::Utc::now() - start).num_seconds();
                if secs < 60 {
                    format!("{}s", secs)
                } else {
                    format!("{}m {}s", secs / 60, secs % 60)
                }
            }
            _ => "-".to_string(),
        }
    };

    // Build a map of stage name -> result for quick lookup
    let result_map: std::collections::HashMap<String, _> = stage_results
        .into_iter()
        .map(|r| (r.stage_name.clone(), r))
        .collect();

    // Convert to StageView, merging definitions with results
    let mut stages: Vec<StageView> = stage_definitions
        .into_iter()
        .map(|def| {
            let result = result_map.get(&def.name);
            let (status, duration) = if let Some(r) = result {
                let dur = match (r.started_at, r.finished_at) {
                    (Some(start), Some(end)) => {
                        let secs = (end - start).num_seconds();
                        if secs < 60 {
                            format!("{}s", secs)
                        } else {
                            format!("{}m {}s", secs / 60, secs % 60)
                        }
                    }
                    (Some(start), None) => {
                        let secs = (chrono::Utc::now() - start).num_seconds();
                        if secs < 60 {
                            format!("{}s", secs)
                        } else {
                            format!("{}m {}s", secs / 60, secs % 60)
                        }
                    }
                    _ => "-".to_string(),
                };
                (r.status.clone(), dur)
            } else {
                ("pending".to_string(), "-".to_string())
            };

            StageView {
                name: def.name,
                status,
                duration,
                dependencies: def.depends_on,
                column: 0,
                row: 0,
                x: 0,
                y: 0,
            }
        })
        .collect();

    // Compute DAG layout
    let (edges, dag_width, dag_height) = compute_dag_layout(&mut stages);

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
            duration: run_duration,
        },
        stages,
        edges,
        first_stage_name,
        dag_width,
        dag_height,
    };

    Ok(Html(template.render().unwrap()))
}

async fn runs_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // Get default tenant
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let pipeline_records = state.pipeline_repo.list_by_tenant(tenant_id).await?;

    let mut pipelines = Vec::new();
    let mut all_runs = Vec::new();

    for p in pipeline_records {
        pipelines.push(PipelineView {
            id: p.id.to_string(),
            name: p.name.clone(),
            repository: p.repository.clone(),
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
            last_run_ago: String::new(),
        });

        let runs = state
            .pipeline_repo
            .list_runs(ResourceId::from_uuid(p.id), 10)
            .await
            .unwrap_or_default();

        for r in runs {
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

            all_runs.push(AllRunView {
                id: r.id.to_string(),
                pipeline_id: p.id.to_string(),
                pipeline_name: p.name.clone(),
                number: r.number,
                status: r.status,
                branch,
                commit_sha,
                commit_message,
                created_at: format_time_ago(r.created_at),
                duration: "1m 23s".to_string(),
            });
        }
    }

    // Sort by created_at (most recent first) - already sorted from DB
    let has_runs = !all_runs.is_empty();
    let template = RunsTemplate {
        pipelines,
        runs: all_runs,
        has_runs,
    };

    Ok(Html(template.render().unwrap()))
}

async fn environments_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let env_records = state.deployment_repo.list_environments(tenant_id).await?;

    let mut environments = Vec::new();
    for env in env_records {
        let service_count = state
            .deployment_repo
            .count_services_in_environment(ResourceId::from_uuid(env.id))
            .await
            .unwrap_or(0) as i32;

        environments.push(EnvironmentView {
            name: env.name,
            service_count,
            health_status: env.health_status,
            target_name: env.target_name,
            target_type: env.target_type,
            last_deploy_ago: "recently".to_string(), // TODO: calculate from deployments
        });
    }

    let has_environments = !environments.is_empty();
    let template = EnvironmentsTemplate {
        environments,
        has_environments,
    };
    Ok(Html(template.render().unwrap()))
}

async fn services_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let service_records = state.deployment_repo.list_services(tenant_id).await?;

    let mut services = Vec::new();
    for svc in service_records {
        let environments = state
            .deployment_repo
            .get_service_environments(ResourceId::from_uuid(svc.id))
            .await
            .unwrap_or_default();

        let last_deploy = state
            .deployment_repo
            .get_service_last_deploy(ResourceId::from_uuid(svc.id))
            .await
            .ok()
            .flatten();

        services.push(ServiceView {
            name: svc.name,
            image: svc.image.unwrap_or_default(),
            status: svc.status,
            environments,
            last_deploy_ago: last_deploy
                .map(format_time_ago)
                .unwrap_or_else(|| "never".to_string()),
        });
    }

    let has_services = !services.is_empty();
    let template = ServicesTemplate {
        services,
        has_services,
    };
    Ok(Html(template.render().unwrap()))
}

async fn history_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let deploy_records = state
        .deployment_repo
        .list_deployments(tenant_id, 50)
        .await?;

    let deployments: Vec<DeploymentView> = deploy_records
        .into_iter()
        .map(|d| {
            let duration = match (d.started_at, d.finished_at) {
                (Some(start), Some(end)) => {
                    let secs = (end - start).num_seconds();
                    if secs < 60 {
                        format!("{}s", secs)
                    } else {
                        format!("{}m {}s", secs / 60, secs % 60)
                    }
                }
                _ => "-".to_string(),
            };

            DeploymentView {
                version: d.version,
                commit_sha: d.commit_sha.unwrap_or_default(),
                service_name: d.service_name,
                environment: d.environment_name,
                status: d.status,
                deployed_ago: format_time_ago(d.created_at),
                duration,
            }
        })
        .collect();

    let has_deployments = !deployments.is_empty();
    let template = HistoryTemplate {
        deployments,
        has_deployments,
    };
    Ok(Html(template.render().unwrap()))
}

async fn targets_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let target_records = state.deployment_repo.list_targets(tenant_id).await?;
    let env_records = state.deployment_repo.list_environments(tenant_id).await?;

    let targets: Vec<TargetView> = target_records
        .into_iter()
        .map(|t| {
            let environment_count =
                env_records.iter().filter(|e| e.target_id == t.id).count() as i32;

            TargetView {
                name: t.name,
                target_type: t.target_type,
                status: t.status,
                region: t.region.unwrap_or_else(|| "-".to_string()),
                environment_count,
            }
        })
        .collect();

    let has_targets = !targets.is_empty();
    let template = TargetsTemplate {
        targets,
        has_targets,
    };
    Ok(Html(template.render().unwrap()))
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

async fn settings_team_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    use buildit_db::OrganizationRepo;

    // Get the first organization (demo purposes)
    let orgs = state.organization_repo.list_organizations().await?;
    let org = orgs
        .first()
        .ok_or_else(|| ApiError::Internal("No organization found".to_string()))?;

    let members_db = state
        .organization_repo
        .list_org_members(ResourceId::from_uuid(org.id))
        .await?;

    let members: Vec<TeamMemberView> = members_db
        .into_iter()
        .map(|m| {
            let initials: String = m
                .user_name
                .split_whitespace()
                .filter_map(|w| w.chars().next())
                .take(2)
                .collect::<String>()
                .to_uppercase();

            TeamMemberView {
                name: m.user_name,
                email: m.user_email,
                role: m.role,
                initials,
            }
        })
        .collect();

    let template = SettingsTeamTemplate { members };
    Ok(Html(template.render().unwrap()))
}

async fn settings_secrets_page(_state: State<AppState>) -> Result<impl IntoResponse, ApiError> {
    // TODO: Load secrets from database when secrets table is created
    let secrets: Vec<SecretView> = vec![
        SecretView {
            name: "DOCKER_PASSWORD".to_string(),
            updated_at: "2 days ago".to_string(),
        },
        SecretView {
            name: "AWS_ACCESS_KEY_ID".to_string(),
            updated_at: "1 week ago".to_string(),
        },
        SecretView {
            name: "AWS_SECRET_ACCESS_KEY".to_string(),
            updated_at: "1 week ago".to_string(),
        },
    ];

    let template = SettingsSecretsTemplate { secrets };
    Ok(Html(template.render().unwrap()))
}

async fn settings_tokens_page(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    use buildit_db::OrganizationRepo;

    let orgs = state.organization_repo.list_organizations().await?;
    let org = orgs
        .first()
        .ok_or_else(|| ApiError::Internal("No organization found".to_string()))?;

    let api_keys = state
        .organization_repo
        .list_api_keys(ResourceId::from_uuid(org.id))
        .await?;

    let tokens: Vec<TokenView> = api_keys
        .into_iter()
        .map(|k| TokenView {
            name: k.name,
            prefix: k.key_prefix,
            last_used: k.last_used_at.map(format_time_ago).unwrap_or_default(),
            expires: k.expires_at.map(format_time_ago).unwrap_or_default(),
        })
        .collect();

    let template = SettingsTokensTemplate { tokens };
    Ok(Html(template.render().unwrap()))
}

async fn settings_git_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    use buildit_db::OrganizationRepo;

    let orgs = state.organization_repo.list_organizations().await?;
    let org = orgs
        .first()
        .ok_or_else(|| ApiError::Internal("No organization found".to_string()))?;

    // TODO: Load actual OAuth connections from database
    let template = SettingsGitTemplate {
        org_id: org.id.to_string(),
        github_connected: false,
        github_username: String::new(),
        gitlab_connected: false,
        gitlab_username: String::new(),
        bitbucket_connected: false,
        bitbucket_username: String::new(),
    };

    Ok(Html(template.render().unwrap()))
}

async fn settings_notifications_page(
    _state: State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    // TODO: Load actual notification settings from database
    let template = SettingsNotificationsTemplate {
        slack_connected: false,
        slack_channel: String::new(),
        has_webhooks: false,
        webhook_count: 0,
    };

    Ok(Html(template.render().unwrap()))
}

// ============================================================================
// Helpers
// ============================================================================

/// Compute DAG layout for stages using an improved algorithm.
/// Returns (edges, width, height) and mutates stages to set x/y positions.
///
/// Layout algorithm:
/// 1. Topological sort to determine level (column) for each stage
/// 2. Center nodes vertically within each level based on their dependencies
/// 3. Route edges with control point offsets to avoid overlap
fn compute_dag_layout(stages: &mut [StageView]) -> (Vec<DagEdge>, i32, i32) {
    use std::collections::{HashMap, HashSet};

    const NODE_WIDTH: i32 = 140;
    const NODE_HEIGHT: i32 = 60;
    const H_SPACING: i32 = 100;
    const V_SPACING: i32 = 50;
    const PADDING: i32 = 40;

    if stages.is_empty() {
        return (vec![], 200, 120);
    }

    // Build name -> index mapping
    let name_to_idx: HashMap<String, usize> = stages
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.clone(), i))
        .collect();

    // Clone data we need to avoid borrow issues
    let deps_list: Vec<Vec<String>> = stages.iter().map(|s| s.dependencies.clone()).collect();
    let status_list: Vec<String> = stages.iter().map(|s| s.status.clone()).collect();
    let name_list: Vec<String> = stages.iter().map(|s| s.name.clone()).collect();

    // Compute levels (topological ordering)
    let mut levels: Vec<i32> = vec![-1; stages.len()];

    fn calc_level(
        idx: usize,
        deps_list: &[Vec<String>],
        name_to_idx: &HashMap<String, usize>,
        levels: &mut Vec<i32>,
        visiting: &mut HashSet<usize>,
    ) -> i32 {
        if levels[idx] >= 0 {
            return levels[idx];
        }
        if visiting.contains(&idx) {
            return 0; // cycle detected
        }
        visiting.insert(idx);

        let deps = &deps_list[idx];
        let level = if deps.is_empty() {
            0
        } else {
            deps.iter()
                .filter_map(|d| name_to_idx.get(d))
                .map(|&di| calc_level(di, deps_list, name_to_idx, levels, visiting))
                .max()
                .unwrap_or(0)
                + 1
        };
        levels[idx] = level;
        level
    }

    for i in 0..stages.len() {
        let mut visiting = HashSet::new();
        calc_level(i, &deps_list, &name_to_idx, &mut levels, &mut visiting);
    }

    // Group by level
    let max_level = *levels.iter().max().unwrap_or(&0);
    let mut by_level: Vec<Vec<usize>> = vec![vec![]; (max_level + 1) as usize];
    for (i, &lvl) in levels.iter().enumerate() {
        by_level[lvl as usize].push(i);
    }

    // Find the maximum number of nodes at any level (for vertical centering)
    let max_nodes_in_level = by_level.iter().map(|v| v.len()).max().unwrap_or(1);

    // Compute positions with improved vertical centering
    let mut positions: Vec<(i32, i32)> = vec![(0, 0); stages.len()];
    let total_height =
        PADDING * 2 + (max_nodes_in_level as i32 - 1) * (NODE_HEIGHT + V_SPACING) + NODE_HEIGHT;

    for (lvl, indices) in by_level.iter().enumerate() {
        let x = PADDING + (lvl as i32) * (NODE_WIDTH + H_SPACING);
        let nodes_in_level = indices.len() as i32;

        // Calculate starting Y to center this level's nodes
        let level_height = (nodes_in_level - 1) * (NODE_HEIGHT + V_SPACING) + NODE_HEIGHT;
        let start_y = (total_height - level_height) / 2;

        for (i, &idx) in indices.iter().enumerate() {
            let y = start_y + (i as i32) * (NODE_HEIGHT + V_SPACING);
            positions[idx] = (x, y);
        }
    }

    // Apply positions to stages
    for (idx, &(x, y)) in positions.iter().enumerate() {
        stages[idx].x = x;
        stages[idx].y = y;
    }

    // Build edges with routing information
    // Track edges per source node for offset calculation
    let mut edges_from: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, deps) in deps_list.iter().enumerate() {
        for dep_name in deps {
            if let Some(&dep_idx) = name_to_idx.get(dep_name) {
                edges_from.entry(dep_idx).or_default().push(idx);
            }
        }
    }

    let mut edges = Vec::new();
    for (idx, deps) in deps_list.iter().enumerate() {
        let (to_x, to_y) = positions[idx];

        for (dep_i, dep_name) in deps.iter().enumerate() {
            if let Some(&dep_idx) = name_to_idx.get(dep_name) {
                let (from_x, from_y) = positions[dep_idx];

                // Calculate control offset to spread out edges from same source
                let outgoing_edges = edges_from.get(&dep_idx).map(|v| v.len()).unwrap_or(1);
                let control_offset = if outgoing_edges > 1 {
                    // Spread edges vertically based on index
                    let spread = 20i32;
                    (dep_i as i32 - (outgoing_edges as i32 / 2)) * spread
                } else {
                    0
                };

                edges.push(DagEdge {
                    from_x: from_x + NODE_WIDTH,
                    from_y: from_y + NODE_HEIGHT / 2,
                    to_x,
                    to_y: to_y + NODE_HEIGHT / 2,
                    from_status: status_list[dep_idx].clone(),
                    from_name: name_list[dep_idx].clone(),
                    to_name: name_list[idx].clone(),
                    control_offset,
                });
            }
        }
    }

    let width = PADDING * 2 + (max_level + 1) * NODE_WIDTH + max_level * H_SPACING;
    let height = total_height.max(160);

    (edges, width.max(200), height)
}

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
