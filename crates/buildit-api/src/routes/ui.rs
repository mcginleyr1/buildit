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
use buildit_db::{
    ApplicationRepo, DeploymentRepo, PipelineRepo, RepositoryRepo, StackRepo, TenantRepo,
};

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
    activities: Vec<ActivityView>,
    has_activity: bool,
}

struct ActivityView {
    #[allow(dead_code)]
    r#type: String,
    message: String,
    ago: String,
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

#[derive(Template)]
#[template(path = "pages/repositories/list.html")]
struct RepositoriesTemplate {
    repositories: Vec<RepositoryView>,
    has_repositories: bool,
}

#[derive(Template)]
#[template(path = "pages/repositories/detail.html")]
struct RepositoryDetailTemplate {
    repository: RepositoryView,
    detected: DetectedConfigView,
    pipelines: Vec<RepoPipelineView>,
    has_pipelines: bool,
    stacks: Vec<RepoStackView>,
    has_stacks: bool,
    webhook_url: String,
}

#[derive(Template)]
#[template(path = "pages/stacks/list.html")]
struct StacksTemplate {
    stacks: Vec<StackView>,
    has_stacks: bool,
}

#[derive(Template)]
#[template(path = "pages/stacks/detail.html")]
struct StackDetailTemplate {
    stack: StackView,
    runs: Vec<StackRunView>,
    has_runs: bool,
    variables: Vec<StackVariableView>,
    has_variables: bool,
}

#[derive(Template)]
#[template(path = "pages/applications/list.html")]
struct ApplicationsTemplate {
    applications: Vec<ApplicationView>,
    has_applications: bool,
}

#[derive(Template)]
#[template(path = "pages/applications/detail.html")]
struct ApplicationDetailTemplate {
    application: ApplicationView,
    resources: Vec<AppResourceView>,
    has_resources: bool,
    syncs: Vec<AppSyncView>,
    has_syncs: bool,
    resource_count: i32,
}

#[derive(Template)]
#[template(path = "pages/applications/new.html")]
struct NewApplicationTemplate {
    repositories: Vec<RepoSelectView>,
    environments: Vec<EnvSelectView>,
}

struct RepoSelectView {
    id: String,
    full_name: String,
}

struct EnvSelectView {
    id: String,
    name: String,
}

#[derive(Template)]
#[template(path = "pages/stacks/new.html")]
struct NewStackTemplate {
    repositories: Vec<RepoSelectView>,
    environments: Vec<EnvSelectView>,
}

#[derive(Template)]
#[template(path = "pages/environments/new.html")]
struct NewEnvironmentTemplate {
    targets: Vec<TargetSelectView>,
}

struct TargetSelectView {
    id: String,
    name: String,
    target_type: String,
}

#[derive(Template)]
#[template(path = "pages/infrastructure/targets_new.html")]
struct NewTargetTemplate {}

#[derive(Template)]
#[template(path = "pages/repositories/connect.html")]
struct ConnectRepositoryTemplate {}

// ============================================================================
// View models
// ============================================================================

struct PipelineView {
    id: String,
    name: String,
    repository: String,
    default_branch: String,
    config: String,
    last_run_id: String,
    last_run_number: i64,
    last_run_status: String,
    last_run_ago: String,
    total_runs: i64,
    success_rate: i64,
    avg_duration: String,
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
    stages: Vec<RunStageView>,
}

/// Minimal stage info for run list display
struct RunStageView {
    name: String,
    status: String,
}

struct RecentRunView {
    pipeline_id: String,
    pipeline_name: String,
    run_id: String,
    run_number: i64,
    status: String,
    ago: String,
    branch: String,
    commit_message: String,
    duration: String,
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

struct RepositoryView {
    id: String,
    provider: String,
    provider_display: String,
    owner: String,
    name: String,
    full_name: String,
    clone_url: String,
    default_branch: String,
    is_private: bool,
    last_synced_ago: String,
    has_pipeline: bool,
    has_terraform: bool,
    has_kubernetes: bool,
    has_dockerfile: bool,
    pipeline_count: i64,
    stack_count: i64,
}

struct DetectedConfigView {
    buildit_config: String,
    has_pipeline: bool,
    has_terraform: bool,
    has_kubernetes: bool,
    has_dockerfile: bool,
    terraform_dir_count: usize,
    kubernetes_file_count: usize,
    dockerfile_count: usize,
}

struct RepoPipelineView {
    id: String,
    name: String,
    last_status: String,
    last_run_ago: String,
}

struct RepoStackView {
    id: String,
    name: String,
    path: String,
    status: String,
}

struct StackView {
    id: String,
    name: String,
    has_description: bool,
    description: String,
    path: String,
    terraform_version: String,
    auto_apply: bool,
    status: String,
    resource_count: i32,
    has_last_run: bool,
    last_run_status: String,
    last_run_type: String,
    last_run_ago: String,
    has_repository: bool,
    repository_name: String,
}

struct StackRunView {
    id: String,
    run_type: String,
    status: String,
    trigger_type: String,
    has_commit_sha: bool,
    commit_sha: String,
    has_changes: bool,
    resources_to_add: i32,
    resources_to_change: i32,
    resources_to_destroy: i32,
    created_at: String,
    duration: String,
    has_plan_output: bool,
}

struct StackVariableView {
    key: String,
    value: String,
    is_sensitive: bool,
    is_hcl: bool,
    description: String,
}

struct ApplicationView {
    id: String,
    name: String,
    has_description: bool,
    description: String,
    path: String,
    target_namespace: String,
    sync_policy: String,
    prune: bool,
    sync_status: String,
    health_status: String,
    has_last_sync: bool,
    last_synced_ago: String,
}

struct AppSyncView {
    id: String,
    revision_short: String,
    status: String,
    trigger_type: String,
    has_changes: bool,
    resources_created: i32,
    resources_updated: i32,
    resources_deleted: i32,
    created_at: String,
}

struct AppResourceView {
    kind: String,
    name: String,
    namespace: String,
    status: String,
    health_status: String,
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
        .route("/environments/new", get(new_environment_page))
        .route("/services", get(services_page))
        .route("/history", get(history_page))
        // Infrastructure
        .route("/targets", get(targets_page))
        .route("/targets/new", get(new_target_page))
        // Repositories
        .route("/repositories", get(repositories_page))
        .route("/repositories/connect", get(connect_repository_page))
        .route("/repositories/{id}", get(repository_detail_page))
        // Stacks
        .route("/stacks", get(stacks_page))
        .route("/stacks/new", get(new_stack_page))
        .route("/stacks/{id}", get(stack_detail_page))
        // Applications
        .route("/applications", get(applications_page))
        .route("/applications/new", get(new_application_page))
        .route("/applications/{id}", get(application_detail_page))
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
                let branch = run
                    .trigger_info
                    .get("branch")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let commit_message = run
                    .trigger_info
                    .get("commit_message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                recent_runs.push(RecentRunView {
                    pipeline_id: p.id.to_string(),
                    pipeline_name: p.name.clone(),
                    run_id: run.id.to_string(),
                    run_number: run.number,
                    status: run.status,
                    ago: format_time_ago(run.created_at),
                    branch,
                    commit_message,
                    duration: String::from("--"),
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
    let activities: Vec<ActivityView> = Vec::new(); // TODO: Populate from actual activity
    let has_activity = !activities.is_empty();
    let template = DashboardTemplate {
        pipeline_count,
        run_count_today: total_runs,
        success_rate,
        recent_runs,
        has_recent_runs,
        activities,
        has_activity,
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
            default_branch: String::from("main"),
            config: String::new(),
            last_run_id,
            last_run_number,
            last_run_status,
            last_run_ago,
            total_runs: 0,
            success_rate: 0,
            avg_duration: String::from("--"),
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
                stages: Vec::new(),             // Stages not loaded in list view
            }
        })
        .collect();

    let has_runs = !runs.is_empty();
    let config_str = serde_json::to_string_pretty(&pipeline.config).unwrap_or_default();
    let template = PipelineDetailTemplate {
        pipeline: PipelineView {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            repository: pipeline.repository,
            default_branch: String::from("main"),
            config: config_str,
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
            last_run_ago: String::new(),
            total_runs: runs.len() as i64,
            success_rate: 0,
            avg_duration: String::from("--"),
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
    let run_stages: Vec<RunStageView> = stages
        .iter()
        .map(|s| RunStageView {
            name: s.name.clone(),
            status: s.status.clone(),
        })
        .collect();
    let template = RunDetailTemplate {
        pipeline: PipelineView {
            id: pipeline.id.to_string(),
            name: pipeline.name,
            repository: pipeline.repository,
            default_branch: String::from("main"),
            config: String::new(),
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
            last_run_ago: String::new(),
            total_runs: 0,
            success_rate: 0,
            avg_duration: String::from("--"),
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
            stages: run_stages,
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
            default_branch: String::from("main"),
            config: String::new(),
            last_run_id: String::new(),
            last_run_number: 0,
            last_run_status: String::new(),
            last_run_ago: String::new(),
            total_runs: 0,
            success_rate: 0,
            avg_duration: String::from("--"),
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

async fn new_environment_page(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let target_records = state
        .deployment_repo
        .list_targets(ResourceId::from_uuid(tenant.id))
        .await?;

    let targets: Vec<TargetSelectView> = target_records
        .into_iter()
        .map(|t| TargetSelectView {
            id: t.id.to_string(),
            name: t.name,
            target_type: t.target_type,
        })
        .collect();

    let template = NewEnvironmentTemplate { targets };
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

async fn new_target_page() -> Result<impl IntoResponse, ApiError> {
    let template = NewTargetTemplate {};
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

async fn repositories_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    use buildit_db::OrganizationRepo;

    // Get the first organization (demo purposes)
    let orgs = state.organization_repo.list_organizations().await?;
    let org = orgs
        .first()
        .ok_or_else(|| ApiError::Internal("No organization found".to_string()))?;

    let repos = state
        .repository_repo
        .list_by_organization(ResourceId::from_uuid(org.id))
        .await?;

    let mut repositories = Vec::new();
    for repo in repos {
        let detected = &repo.detected_config;

        // Count pipelines linked to this repo
        let pipeline_count = state
            .pipeline_repo
            .list_by_repository(ResourceId::from_uuid(repo.id))
            .await
            .map(|p| p.len() as i64)
            .unwrap_or(0);

        // Count stacks linked to this repo
        let stack_count = state
            .stack_repo
            .list_stacks_by_repository(ResourceId::from_uuid(repo.id))
            .await
            .map(|s| s.len() as i64)
            .unwrap_or(0);

        let provider_str = repo.provider.to_string();
        let provider_display = capitalize_first(&provider_str);
        repositories.push(RepositoryView {
            id: repo.id.to_string(),
            provider: provider_str,
            provider_display,
            owner: repo.owner.clone(),
            name: repo.name.clone(),
            full_name: repo.full_name.clone(),
            clone_url: repo.clone_url.clone(),
            default_branch: repo.default_branch.clone(),
            is_private: repo.is_private,
            last_synced_ago: repo
                .last_synced_at
                .map(format_time_ago)
                .unwrap_or_else(|| "never".to_string()),
            has_pipeline: detected.has_pipeline(),
            has_terraform: detected.has_terraform(),
            has_kubernetes: detected.has_kubernetes(),
            has_dockerfile: !detected.dockerfiles.is_empty(),
            pipeline_count,
            stack_count,
        });
    }

    let has_repositories = !repositories.is_empty();
    let template = RepositoriesTemplate {
        repositories,
        has_repositories,
    };

    Ok(Html(template.render().unwrap()))
}

async fn connect_repository_page() -> Result<impl IntoResponse, ApiError> {
    let template = ConnectRepositoryTemplate {};
    Ok(Html(template.render().unwrap()))
}

async fn repository_detail_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let repo = state
        .repository_repo
        .get_by_id(ResourceId::from_uuid(id))
        .await?;

    let detected = &repo.detected_config;

    // Get pipelines linked to this repo
    let pipeline_records = state
        .pipeline_repo
        .list_by_repository(ResourceId::from_uuid(repo.id))
        .await?;

    let mut pipelines = Vec::new();
    for p in pipeline_records {
        let runs = state
            .pipeline_repo
            .list_runs(ResourceId::from_uuid(p.id), 1)
            .await
            .unwrap_or_default();
        let last_run = runs.first();

        pipelines.push(RepoPipelineView {
            id: p.id.to_string(),
            name: p.name,
            last_status: last_run.map(|r| r.status.clone()).unwrap_or_default(),
            last_run_ago: last_run
                .map(|r| format_time_ago(r.created_at))
                .unwrap_or_else(|| "never".to_string()),
        });
    }

    // Get stacks linked to this repo
    let stack_records = state
        .stack_repo
        .list_stacks_by_repository(ResourceId::from_uuid(repo.id))
        .await?;

    let stacks: Vec<RepoStackView> = stack_records
        .into_iter()
        .map(|s| RepoStackView {
            id: s.id.to_string(),
            name: s.name,
            path: s.path,
            status: s.status.to_string(),
        })
        .collect();

    let webhook_url = format!("https://api.buildit.dev/webhooks/github/{}", repo.id);

    let provider_str = repo.provider.to_string();
    let provider_display = capitalize_first(&provider_str);
    let repository = RepositoryView {
        id: repo.id.to_string(),
        provider: provider_str,
        provider_display,
        owner: repo.owner.clone(),
        name: repo.name.clone(),
        full_name: repo.full_name.clone(),
        clone_url: repo.clone_url.clone(),
        default_branch: repo.default_branch.clone(),
        is_private: repo.is_private,
        last_synced_ago: repo
            .last_synced_at
            .map(format_time_ago)
            .unwrap_or_else(|| "never".to_string()),
        has_pipeline: detected.has_pipeline(),
        has_terraform: detected.has_terraform(),
        has_kubernetes: detected.has_kubernetes(),
        has_dockerfile: !detected.dockerfiles.is_empty(),
        pipeline_count: pipelines.len() as i64,
        stack_count: stacks.len() as i64,
    };

    let detected_view = DetectedConfigView {
        buildit_config: detected.buildit_config.clone().unwrap_or_default(),
        has_pipeline: detected.has_pipeline(),
        has_terraform: detected.has_terraform(),
        has_kubernetes: detected.has_kubernetes(),
        has_dockerfile: !detected.dockerfiles.is_empty(),
        terraform_dir_count: detected.terraform_dirs.len(),
        kubernetes_file_count: detected.kubernetes_files.len(),
        dockerfile_count: detected.dockerfiles.len(),
    };

    let has_pipelines = !pipelines.is_empty();
    let has_stacks = !stacks.is_empty();

    let template = RepositoryDetailTemplate {
        repository,
        detected: detected_view,
        pipelines,
        has_pipelines,
        stacks,
        has_stacks,
        webhook_url,
    };

    Ok(Html(template.render().unwrap()))
}

async fn stacks_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let stack_records = state.stack_repo.list_stacks_by_tenant(tenant_id).await?;

    let mut stacks = Vec::new();
    for s in stack_records {
        // Get last run for this stack
        let runs = state
            .stack_repo
            .list_runs(ResourceId::from_uuid(s.id), 1)
            .await
            .unwrap_or_default();
        let last_run = runs.first();

        // Get repository name if linked
        let repository_name = if let Some(repo_id) = s.repository_id {
            state
                .repository_repo
                .get_by_id(ResourceId::from_uuid(repo_id))
                .await
                .map(|r| r.full_name)
                .unwrap_or_default()
        } else {
            String::new()
        };

        // Count resources from state (if available)
        let resource_count = state
            .stack_repo
            .get_state(ResourceId::from_uuid(s.id))
            .await
            .ok()
            .flatten()
            .and_then(|state| {
                state
                    .state_json
                    .get("resources")
                    .and_then(|r| r.as_array().map(|arr| arr.len() as i32))
            })
            .unwrap_or(0);

        let has_description = s.description.is_some();
        let has_repository = !repository_name.is_empty();
        stacks.push(StackView {
            id: s.id.to_string(),
            name: s.name,
            has_description,
            description: s.description.unwrap_or_default(),
            path: s.path,
            terraform_version: s.terraform_version,
            auto_apply: s.auto_apply,
            status: s.status.to_string(),
            resource_count,
            has_last_run: last_run.is_some(),
            last_run_status: last_run.map(|r| r.status.to_string()).unwrap_or_default(),
            last_run_type: last_run.map(|r| r.run_type.to_string()).unwrap_or_default(),
            last_run_ago: last_run
                .map(|r| format_time_ago(r.created_at))
                .unwrap_or_default(),
            has_repository,
            repository_name,
        });
    }

    let has_stacks = !stacks.is_empty();
    let template = StacksTemplate { stacks, has_stacks };

    Ok(Html(template.render().unwrap()))
}

async fn new_stack_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    use buildit_db::OrganizationRepo;

    // Get repositories
    let orgs = state.organization_repo.list_organizations().await?;
    let org = orgs
        .first()
        .ok_or_else(|| ApiError::Internal("No organization found".to_string()))?;

    let repos = state
        .repository_repo
        .list_by_organization(ResourceId::from_uuid(org.id))
        .await?;

    let repositories: Vec<RepoSelectView> = repos
        .into_iter()
        .map(|r| RepoSelectView {
            id: r.id.to_string(),
            full_name: r.full_name,
        })
        .collect();

    // Get environments
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let env_records = state
        .deployment_repo
        .list_environments(ResourceId::from_uuid(tenant.id))
        .await?;

    let environments: Vec<EnvSelectView> = env_records
        .into_iter()
        .map(|e| EnvSelectView {
            id: e.id.to_string(),
            name: e.name,
        })
        .collect();

    let template = NewStackTemplate {
        repositories,
        environments,
    };

    Ok(Html(template.render().unwrap()))
}

async fn stack_detail_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let s = state
        .stack_repo
        .get_stack(ResourceId::from_uuid(id))
        .await?;

    // Get runs
    let run_records = state
        .stack_repo
        .list_runs(ResourceId::from_uuid(id), 20)
        .await?;

    let runs: Vec<StackRunView> = run_records
        .into_iter()
        .map(|r| {
            let duration = match (r.started_at, r.finished_at) {
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
                    format!("{}s (running)", secs)
                }
                _ => "-".to_string(),
            };

            StackRunView {
                id: r.id.to_string(),
                run_type: r.run_type.to_string(),
                status: r.status.to_string(),
                trigger_type: r.trigger_type.to_string(),
                has_commit_sha: r.commit_sha.is_some(),
                commit_sha: r.commit_sha.unwrap_or_default().chars().take(7).collect(),
                has_changes: r.resources_to_add > 0
                    || r.resources_to_change > 0
                    || r.resources_to_destroy > 0,
                resources_to_add: r.resources_to_add,
                resources_to_change: r.resources_to_change,
                resources_to_destroy: r.resources_to_destroy,
                created_at: format_time_ago(r.created_at),
                duration,
                has_plan_output: r.plan_output.is_some(),
            }
        })
        .collect();

    // Get variables
    let variable_records = state
        .stack_repo
        .list_variables(ResourceId::from_uuid(id))
        .await?;

    let variables: Vec<StackVariableView> = variable_records
        .into_iter()
        .map(|v| StackVariableView {
            key: v.key,
            value: if v.is_sensitive {
                "********".to_string()
            } else {
                v.value.unwrap_or_default()
            },
            is_sensitive: v.is_sensitive,
            is_hcl: v.is_hcl,
            description: v.description.unwrap_or_default(),
        })
        .collect();

    // Get repository name if linked
    let repository_name = if let Some(repo_id) = s.repository_id {
        state
            .repository_repo
            .get_by_id(ResourceId::from_uuid(repo_id))
            .await
            .map(|r| r.full_name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Count resources from state
    let resource_count = state
        .stack_repo
        .get_state(ResourceId::from_uuid(s.id))
        .await
        .ok()
        .flatten()
        .and_then(|state| {
            state
                .state_json
                .get("resources")
                .and_then(|r| r.as_array().map(|arr| arr.len() as i32))
        })
        .unwrap_or(0);

    let last_run = runs.first();
    let has_description = s.description.is_some();
    let has_repository = !repository_name.is_empty();
    let stack = StackView {
        id: s.id.to_string(),
        name: s.name,
        has_description,
        description: s.description.unwrap_or_default(),
        path: s.path,
        terraform_version: s.terraform_version,
        auto_apply: s.auto_apply,
        status: s.status.to_string(),
        resource_count,
        has_last_run: last_run.is_some(),
        last_run_status: last_run.map(|r| r.status.clone()).unwrap_or_default(),
        last_run_type: last_run.map(|r| r.run_type.clone()).unwrap_or_default(),
        last_run_ago: last_run.map(|r| r.created_at.clone()).unwrap_or_default(),
        has_repository,
        repository_name,
    };

    let has_runs = !runs.is_empty();
    let has_variables = !variables.is_empty();

    let template = StackDetailTemplate {
        stack,
        runs,
        has_runs,
        variables,
        has_variables,
    };

    Ok(Html(template.render().unwrap()))
}

async fn applications_page(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let tenant_id = ResourceId::from_uuid(tenant.id);
    let app_records = state
        .application_repo
        .list_applications_by_tenant(tenant_id)
        .await?;

    let applications: Vec<ApplicationView> = app_records
        .into_iter()
        .map(|a| {
            let has_last_sync = a.last_synced_at.is_some();
            let last_synced_ago = a.last_synced_at.map(format_time_ago).unwrap_or_default();

            ApplicationView {
                id: a.id.to_string(),
                name: a.name,
                has_description: a.description.is_some(),
                description: a.description.unwrap_or_default(),
                path: a.path,
                target_namespace: a.target_namespace,
                sync_policy: a.sync_policy.to_string(),
                prune: a.prune,
                sync_status: a.sync_status.to_string(),
                health_status: a.health_status.to_string(),
                has_last_sync,
                last_synced_ago,
            }
        })
        .collect();

    let has_applications = !applications.is_empty();
    let template = ApplicationsTemplate {
        applications,
        has_applications,
    };

    Ok(Html(template.render().unwrap()))
}

async fn new_application_page(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    use buildit_db::OrganizationRepo;

    // Get repositories
    let orgs = state.organization_repo.list_organizations().await?;
    let org = orgs
        .first()
        .ok_or_else(|| ApiError::Internal("No organization found".to_string()))?;

    let repos = state
        .repository_repo
        .list_by_organization(ResourceId::from_uuid(org.id))
        .await?;

    let repositories: Vec<RepoSelectView> = repos
        .into_iter()
        .map(|r| RepoSelectView {
            id: r.id.to_string(),
            full_name: r.full_name,
        })
        .collect();

    // Get environments
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let env_records = state
        .deployment_repo
        .list_environments(ResourceId::from_uuid(tenant.id))
        .await?;

    let environments: Vec<EnvSelectView> = env_records
        .into_iter()
        .map(|e| EnvSelectView {
            id: e.id.to_string(),
            name: e.name,
        })
        .collect();

    let template = NewApplicationTemplate {
        repositories,
        environments,
    };

    Ok(Html(template.render().unwrap()))
}

async fn application_detail_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let a = state
        .application_repo
        .get_application(ResourceId::from_uuid(id))
        .await?;

    // Get resources
    let resource_records = state
        .application_repo
        .list_resources(ResourceId::from_uuid(id))
        .await?;

    let resources: Vec<AppResourceView> = resource_records
        .into_iter()
        .map(|r| AppResourceView {
            kind: r.kind,
            name: r.name,
            namespace: r.namespace,
            status: r.status.to_string(),
            health_status: r.health_status.to_string(),
        })
        .collect();

    // Get sync history
    let sync_records = state
        .application_repo
        .list_syncs(ResourceId::from_uuid(id), 20)
        .await?;

    let syncs: Vec<AppSyncView> = sync_records
        .into_iter()
        .map(|s| {
            let has_changes =
                s.resources_created > 0 || s.resources_updated > 0 || s.resources_deleted > 0;

            AppSyncView {
                id: s.id.to_string(),
                revision_short: s.revision.chars().take(7).collect(),
                status: s.status.to_string(),
                trigger_type: s.trigger_type.to_string(),
                has_changes,
                resources_created: s.resources_created,
                resources_updated: s.resources_updated,
                resources_deleted: s.resources_deleted,
                created_at: format_time_ago(s.created_at),
            }
        })
        .collect();

    let has_last_sync = a.last_synced_at.is_some();
    let last_synced_ago = a.last_synced_at.map(format_time_ago).unwrap_or_default();

    let application = ApplicationView {
        id: a.id.to_string(),
        name: a.name,
        has_description: a.description.is_some(),
        description: a.description.unwrap_or_default(),
        path: a.path,
        target_namespace: a.target_namespace,
        sync_policy: a.sync_policy.to_string(),
        prune: a.prune,
        sync_status: a.sync_status.to_string(),
        health_status: a.health_status.to_string(),
        has_last_sync,
        last_synced_ago,
    };

    let resource_count = resources.len() as i32;
    let has_resources = !resources.is_empty();
    let has_syncs = !syncs.is_empty();

    let template = ApplicationDetailTemplate {
        application,
        resources,
        has_resources,
        syncs,
        has_syncs,
        resource_count,
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

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}
