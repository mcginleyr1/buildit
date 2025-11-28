//! Stack (Terraform) management endpoints.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use crate::services::git::GitService;
use crate::services::terraform::TerraformService;
use buildit_core::ResourceId;
use buildit_core::stack::{
    CreateStackRequest, PlanSummary, StackRunStatus, StackRunType, StackStatus, StackTriggerType,
    TriggerStackRunRequest,
};
use buildit_db::{RepositoryRepo, StackRepo};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_stacks).post(create_stack))
        .route("/{id}", get(get_stack).delete(delete_stack))
        .route("/{id}/runs", get(list_runs).post(trigger_run))
        .route("/{id}/runs/{run_id}", get(get_run))
        .route("/{id}/runs/{run_id}/approve", post(approve_run))
        .route("/{id}/variables", get(list_variables).post(set_variable))
}

#[derive(Debug, Deserialize)]
pub struct ListStacksQuery {
    pub tenant_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct StackResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub repository_id: Option<Uuid>,
    pub path: String,
    pub terraform_version: String,
    pub auto_apply: bool,
    pub status: String,
    pub last_run_at: Option<String>,
}

async fn list_stacks(
    State(state): State<AppState>,
    Query(query): Query<ListStacksQuery>,
) -> Result<Json<Vec<StackResponse>>, ApiError> {
    let stacks = state
        .stack_repo
        .list_stacks_by_tenant(ResourceId::from_uuid(query.tenant_id))
        .await?;

    let response: Vec<StackResponse> = stacks
        .into_iter()
        .map(|s| StackResponse {
            id: s.id,
            name: s.name,
            description: s.description,
            repository_id: s.repository_id,
            path: s.path,
            terraform_version: s.terraform_version,
            auto_apply: s.auto_apply,
            status: s.status.to_string(),
            last_run_at: s.last_run_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct CreateStackApiRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub repository_id: Option<Uuid>,
    pub path: Option<String>,
    pub terraform_version: Option<String>,
    pub auto_apply: Option<bool>,
}

async fn create_stack(
    State(state): State<AppState>,
    Json(req): Json<CreateStackApiRequest>,
) -> Result<Json<StackResponse>, ApiError> {
    let stack = state
        .stack_repo
        .create_stack(
            ResourceId::from_uuid(req.tenant_id),
            &req.name,
            req.description.as_deref(),
            req.repository_id.map(ResourceId::from_uuid),
            req.path.as_deref().unwrap_or("."),
            req.terraform_version.as_deref().unwrap_or("1.5.0"),
            req.auto_apply.unwrap_or(false),
        )
        .await?;

    // If linked to a repository, initialize terraform
    if let Some(repo_id) = req.repository_id {
        let repo = state
            .repository_repo
            .get_by_id(ResourceId::from_uuid(repo_id))
            .await?;

        // Clone repo and initialize terraform in background
        let stack_id = stack.id;
        let stack_repo = state.stack_repo.clone();
        let path = req.path.clone().unwrap_or_else(|| ".".to_string());

        tokio::spawn(async move {
            let git_service = GitService::new();
            let tf_service = TerraformService::new();

            // Clone the repository
            match git_service.ensure_cloned(&repo.clone_url, None).await {
                Ok(repo_path) => {
                    let working_dir = repo_path.join(&path);

                    // Update stack with working directory
                    if let Err(e) = stack_repo
                        .update_stack_working_directory(
                            ResourceId::from_uuid(stack_id),
                            working_dir.to_str().unwrap_or(""),
                        )
                        .await
                    {
                        tracing::error!(error = %e, "Failed to update stack working directory");
                        return;
                    }

                    // Run terraform init
                    match tf_service.init(&working_dir, &HashMap::new()).await {
                        Ok(_) => {
                            if let Err(e) = stack_repo
                                .update_stack_status(
                                    ResourceId::from_uuid(stack_id),
                                    StackStatus::Ready,
                                )
                                .await
                            {
                                tracing::error!(error = %e, "Failed to update stack status");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Terraform init failed");
                            let _ = stack_repo
                                .update_stack_status(
                                    ResourceId::from_uuid(stack_id),
                                    StackStatus::Error,
                                )
                                .await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to clone repository");
                    let _ = stack_repo
                        .update_stack_status(ResourceId::from_uuid(stack_id), StackStatus::Error)
                        .await;
                }
            }
        });

        // Update status to initializing
        state
            .stack_repo
            .update_stack_status(ResourceId::from_uuid(stack.id), StackStatus::Initializing)
            .await?;
    }

    Ok(Json(StackResponse {
        id: stack.id,
        name: stack.name,
        description: stack.description,
        repository_id: stack.repository_id,
        path: stack.path,
        terraform_version: stack.terraform_version,
        auto_apply: stack.auto_apply,
        status: stack.status.to_string(),
        last_run_at: stack.last_run_at.map(|t| t.to_rfc3339()),
    }))
}

async fn get_stack(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<StackResponse>, ApiError> {
    let stack = state
        .stack_repo
        .get_stack(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(StackResponse {
        id: stack.id,
        name: stack.name,
        description: stack.description,
        repository_id: stack.repository_id,
        path: stack.path,
        terraform_version: stack.terraform_version,
        auto_apply: stack.auto_apply,
        status: stack.status.to_string(),
        last_run_at: stack.last_run_at.map(|t| t.to_rfc3339()),
    }))
}

async fn delete_stack(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .stack_repo
        .delete_stack(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

#[derive(Debug, Serialize)]
pub struct StackRunResponse {
    pub id: Uuid,
    pub run_type: String,
    pub status: String,
    pub trigger_type: String,
    pub resources_to_add: i32,
    pub resources_to_change: i32,
    pub resources_to_destroy: i32,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub error_message: Option<String>,
}

async fn list_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<StackRunResponse>>, ApiError> {
    let runs = state
        .stack_repo
        .list_runs(ResourceId::from_uuid(id), 20)
        .await?;

    let response: Vec<StackRunResponse> = runs
        .into_iter()
        .map(|r| StackRunResponse {
            id: r.id,
            run_type: format!("{:?}", r.run_type).to_lowercase(),
            status: r.status.to_string(),
            trigger_type: r.trigger_type.to_string(),
            resources_to_add: r.resources_to_add,
            resources_to_change: r.resources_to_change,
            resources_to_destroy: r.resources_to_destroy,
            started_at: r.started_at.map(|t| t.to_rfc3339()),
            finished_at: r.finished_at.map(|t| t.to_rfc3339()),
            error_message: r.error_message,
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct TriggerRunApiRequest {
    pub run_type: String, // "plan", "apply", "destroy"
}

async fn trigger_run(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TriggerRunApiRequest>,
) -> Result<Json<StackRunResponse>, ApiError> {
    let run_type = match req.run_type.as_str() {
        "plan" => StackRunType::Plan,
        "apply" => StackRunType::Apply,
        "destroy" => StackRunType::Destroy,
        "refresh" => StackRunType::Refresh,
        _ => return Err(ApiError::BadRequest("Invalid run type".to_string())),
    };

    let stack = state
        .stack_repo
        .get_stack(ResourceId::from_uuid(id))
        .await?;

    // Create the run record
    let run = state
        .stack_repo
        .create_run(
            ResourceId::from_uuid(id),
            run_type,
            None, // TODO: get user from auth
            StackTriggerType::Manual,
            None,
        )
        .await?;

    // Execute in background
    let stack_repo = state.stack_repo.clone();
    let run_id = run.id;

    tokio::spawn(async move {
        let tf_service = TerraformService::new();

        // Mark as running
        if let Err(e) = stack_repo
            .update_run_started(ResourceId::from_uuid(run_id))
            .await
        {
            tracing::error!(error = %e, "Failed to update run started");
            return;
        }

        let working_dir = match &stack.working_directory {
            Some(dir) => std::path::PathBuf::from(dir),
            None => {
                tracing::error!("Stack has no working directory");
                let _ = stack_repo
                    .update_run_finished(
                        ResourceId::from_uuid(run_id),
                        StackRunStatus::Failed,
                        Some("Stack has no working directory"),
                    )
                    .await;
                return;
            }
        };

        match run_type {
            StackRunType::Plan => {
                match tf_service
                    .plan(&working_dir, &HashMap::new(), None, None)
                    .await
                {
                    Ok(result) => {
                        let plan_json = result.plan_json.clone();
                        let _ = stack_repo
                            .update_run_plan_output(
                                ResourceId::from_uuid(run_id),
                                &result.output,
                                plan_json,
                                result.summary.to_add.len() as i32,
                                result.summary.to_change.len() as i32,
                                result.summary.to_destroy.len() as i32,
                            )
                            .await;

                        let status = if result.has_changes && !stack.auto_apply {
                            StackRunStatus::NeedsApproval
                        } else if result.has_changes && stack.auto_apply {
                            // Auto-apply: run apply immediately
                            if let Some(plan_file) = result.plan_file {
                                match tf_service.apply(&working_dir, &plan_file, None).await {
                                    Ok(apply_result) => {
                                        let _ = stack_repo
                                            .update_run_apply_output(
                                                ResourceId::from_uuid(run_id),
                                                &apply_result.output,
                                            )
                                            .await;
                                        StackRunStatus::Succeeded
                                    }
                                    Err(e) => {
                                        tracing::error!(error = %e, "Apply failed");
                                        StackRunStatus::Failed
                                    }
                                }
                            } else {
                                StackRunStatus::Succeeded
                            }
                        } else {
                            StackRunStatus::Succeeded
                        };

                        let _ = stack_repo
                            .update_run_finished(ResourceId::from_uuid(run_id), status, None)
                            .await;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Plan failed");
                        let _ = stack_repo
                            .update_run_finished(
                                ResourceId::from_uuid(run_id),
                                StackRunStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
            StackRunType::Apply => {
                // For apply, we need to run plan first then apply
                match tf_service
                    .plan(&working_dir, &HashMap::new(), None, None)
                    .await
                {
                    Ok(result) => {
                        if let Some(plan_file) = result.plan_file {
                            match tf_service.apply(&working_dir, &plan_file, None).await {
                                Ok(apply_result) => {
                                    let _ = stack_repo
                                        .update_run_apply_output(
                                            ResourceId::from_uuid(run_id),
                                            &apply_result.output,
                                        )
                                        .await;
                                    let _ = stack_repo
                                        .update_run_finished(
                                            ResourceId::from_uuid(run_id),
                                            StackRunStatus::Succeeded,
                                            None,
                                        )
                                        .await;
                                }
                                Err(e) => {
                                    let _ = stack_repo
                                        .update_run_finished(
                                            ResourceId::from_uuid(run_id),
                                            StackRunStatus::Failed,
                                            Some(&e.to_string()),
                                        )
                                        .await;
                                }
                            }
                        } else {
                            // No changes
                            let _ = stack_repo
                                .update_run_finished(
                                    ResourceId::from_uuid(run_id),
                                    StackRunStatus::Succeeded,
                                    None,
                                )
                                .await;
                        }
                    }
                    Err(e) => {
                        let _ = stack_repo
                            .update_run_finished(
                                ResourceId::from_uuid(run_id),
                                StackRunStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
            StackRunType::Destroy => {
                match tf_service
                    .destroy(&working_dir, &HashMap::new(), None)
                    .await
                {
                    Ok(output) => {
                        let _ = stack_repo
                            .update_run_apply_output(ResourceId::from_uuid(run_id), &output)
                            .await;
                        let _ = stack_repo
                            .update_run_finished(
                                ResourceId::from_uuid(run_id),
                                StackRunStatus::Succeeded,
                                None,
                            )
                            .await;
                    }
                    Err(e) => {
                        let _ = stack_repo
                            .update_run_finished(
                                ResourceId::from_uuid(run_id),
                                StackRunStatus::Failed,
                                Some(&e.to_string()),
                            )
                            .await;
                    }
                }
            }
            StackRunType::Refresh => {
                // TODO: Implement refresh
                let _ = stack_repo
                    .update_run_finished(
                        ResourceId::from_uuid(run_id),
                        StackRunStatus::Succeeded,
                        None,
                    )
                    .await;
            }
        }
    });

    Ok(Json(StackRunResponse {
        id: run.id,
        run_type: format!("{:?}", run.run_type).to_lowercase(),
        status: run.status.to_string(),
        trigger_type: run.trigger_type.to_string(),
        resources_to_add: run.resources_to_add,
        resources_to_change: run.resources_to_change,
        resources_to_destroy: run.resources_to_destroy,
        started_at: run.started_at.map(|t| t.to_rfc3339()),
        finished_at: run.finished_at.map(|t| t.to_rfc3339()),
        error_message: run.error_message,
    }))
}

async fn get_run(
    State(state): State<AppState>,
    Path((stack_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<StackRunResponse>, ApiError> {
    let run = state
        .stack_repo
        .get_run(ResourceId::from_uuid(run_id))
        .await?;

    Ok(Json(StackRunResponse {
        id: run.id,
        run_type: format!("{:?}", run.run_type).to_lowercase(),
        status: run.status.to_string(),
        trigger_type: run.trigger_type.to_string(),
        resources_to_add: run.resources_to_add,
        resources_to_change: run.resources_to_change,
        resources_to_destroy: run.resources_to_destroy,
        started_at: run.started_at.map(|t| t.to_rfc3339()),
        finished_at: run.finished_at.map(|t| t.to_rfc3339()),
        error_message: run.error_message,
    }))
}

async fn approve_run(
    State(state): State<AppState>,
    Path((stack_id, run_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<StackRunResponse>, ApiError> {
    // TODO: Get user from auth
    let user_id = Uuid::nil(); // Placeholder

    // Approve the run
    state
        .stack_repo
        .approve_run(
            ResourceId::from_uuid(run_id),
            ResourceId::from_uuid(user_id),
        )
        .await?;

    // Get the stack and run
    let stack = state
        .stack_repo
        .get_stack(ResourceId::from_uuid(stack_id))
        .await?;
    let run = state
        .stack_repo
        .get_run(ResourceId::from_uuid(run_id))
        .await?;

    // Execute apply in background
    let stack_repo = state.stack_repo.clone();

    tokio::spawn(async move {
        let tf_service = TerraformService::new();

        let working_dir = match &stack.working_directory {
            Some(dir) => std::path::PathBuf::from(dir),
            None => return,
        };

        // Update status to applying
        let _ = stack_repo
            .update_run_status(ResourceId::from_uuid(run_id), StackRunStatus::Applying)
            .await;

        // The plan file should still exist from the original plan
        let plan_file = working_dir.join("tfplan");

        if plan_file.exists() {
            match tf_service.apply(&working_dir, &plan_file, None).await {
                Ok(result) => {
                    let _ = stack_repo
                        .update_run_apply_output(ResourceId::from_uuid(run_id), &result.output)
                        .await;
                    let _ = stack_repo
                        .update_run_finished(
                            ResourceId::from_uuid(run_id),
                            StackRunStatus::Succeeded,
                            None,
                        )
                        .await;
                }
                Err(e) => {
                    let _ = stack_repo
                        .update_run_finished(
                            ResourceId::from_uuid(run_id),
                            StackRunStatus::Failed,
                            Some(&e.to_string()),
                        )
                        .await;
                }
            }
        } else {
            // Plan file doesn't exist, need to re-plan
            let _ = stack_repo
                .update_run_finished(
                    ResourceId::from_uuid(run_id),
                    StackRunStatus::Failed,
                    Some("Plan file not found - please run a new plan"),
                )
                .await;
        }
    });

    Ok(Json(StackRunResponse {
        id: run.id,
        run_type: format!("{:?}", run.run_type).to_lowercase(),
        status: "approved".to_string(),
        trigger_type: run.trigger_type.to_string(),
        resources_to_add: run.resources_to_add,
        resources_to_change: run.resources_to_change,
        resources_to_destroy: run.resources_to_destroy,
        started_at: run.started_at.map(|t| t.to_rfc3339()),
        finished_at: run.finished_at.map(|t| t.to_rfc3339()),
        error_message: run.error_message,
    }))
}

#[derive(Debug, Serialize)]
pub struct StackVariableResponse {
    pub key: String,
    pub value: Option<String>,
    pub is_sensitive: bool,
    pub is_hcl: bool,
    pub description: Option<String>,
}

async fn list_variables(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<StackVariableResponse>>, ApiError> {
    let variables = state
        .stack_repo
        .list_variables(ResourceId::from_uuid(id))
        .await?;

    let response: Vec<StackVariableResponse> = variables
        .into_iter()
        .map(|v| StackVariableResponse {
            key: v.key,
            value: if v.is_sensitive { None } else { v.value },
            is_sensitive: v.is_sensitive,
            is_hcl: v.is_hcl,
            description: v.description,
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct SetVariableRequest {
    pub key: String,
    pub value: Option<String>,
    pub is_sensitive: Option<bool>,
    pub is_hcl: Option<bool>,
    pub description: Option<String>,
}

async fn set_variable(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SetVariableRequest>,
) -> Result<Json<StackVariableResponse>, ApiError> {
    let variable = state
        .stack_repo
        .set_variable(
            ResourceId::from_uuid(id),
            &req.key,
            req.value.as_deref(),
            req.is_sensitive.unwrap_or(false),
            req.is_hcl.unwrap_or(false),
            req.description.as_deref(),
        )
        .await?;

    Ok(Json(StackVariableResponse {
        key: variable.key,
        value: if variable.is_sensitive {
            None
        } else {
            variable.value
        },
        is_sensitive: variable.is_sensitive,
        is_hcl: variable.is_hcl,
        description: variable.description,
    }))
}
