//! Deployment API routes (environments, targets, services).

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use buildit_core::ResourceId;
use buildit_db::{DeploymentRepo, TenantRepo};

pub fn router() -> Router<AppState> {
    Router::new()
        // Environments
        .route(
            "/environments",
            get(list_environments).post(create_environment),
        )
        .route(
            "/environments/{id}",
            get(get_environment).delete(delete_environment),
        )
        // Targets
        .route("/targets", get(list_targets).post(create_target))
        .route("/targets/{id}", get(get_target).delete(delete_target))
}

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub description: Option<String>,
    pub target_id: Uuid,
    #[serde(default)]
    pub requires_approval: bool,
    #[serde(default)]
    pub auto_deploy: bool,
}

#[derive(Debug, Serialize)]
pub struct EnvironmentResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub target_id: Uuid,
    pub target_name: String,
    pub target_type: String,
    pub health_status: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTargetRequest {
    pub name: String,
    pub target_type: String,
    pub region: Option<String>,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct TargetResponse {
    pub id: Uuid,
    pub name: String,
    pub target_type: String,
    pub region: Option<String>,
    pub status: String,
}

// ============================================================================
// Environment handlers
// ============================================================================

async fn list_environments(
    State(state): State<AppState>,
) -> Result<Json<Vec<EnvironmentResponse>>, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let envs = state
        .deployment_repo
        .list_environments(ResourceId::from_uuid(tenant.id))
        .await?;

    let response: Vec<EnvironmentResponse> = envs
        .into_iter()
        .map(|e| EnvironmentResponse {
            id: e.id,
            name: e.name,
            description: None, // TODO: Add description field to DB
            target_id: e.target_id,
            target_name: e.target_name,
            target_type: e.target_type,
            health_status: e.health_status,
        })
        .collect();

    Ok(Json(response))
}

async fn create_environment(
    State(state): State<AppState>,
    Json(req): Json<CreateEnvironmentRequest>,
) -> Result<Json<EnvironmentResponse>, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let env = state
        .deployment_repo
        .create_environment(
            ResourceId::from_uuid(tenant.id),
            ResourceId::from_uuid(req.target_id),
            &req.name,
            serde_json::json!({}),
        )
        .await?;

    // Get target info for response
    let target = state
        .deployment_repo
        .get_target(ResourceId::from_uuid(req.target_id))
        .await?;

    Ok(Json(EnvironmentResponse {
        id: env.id,
        name: env.name,
        description: req.description,
        target_id: target.id,
        target_name: target.name,
        target_type: target.target_type,
        health_status: env.health_status,
    }))
}

async fn get_environment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EnvironmentResponse>, ApiError> {
    let env = state
        .deployment_repo
        .get_environment(ResourceId::from_uuid(id))
        .await?;

    let target = state
        .deployment_repo
        .get_target(ResourceId::from_uuid(env.target_id))
        .await?;

    Ok(Json(EnvironmentResponse {
        id: env.id,
        name: env.name,
        description: None,
        target_id: env.target_id,
        target_name: target.name,
        target_type: target.target_type,
        health_status: env.health_status,
    }))
}

async fn delete_environment(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .deployment_repo
        .delete_environment(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

// ============================================================================
// Target handlers
// ============================================================================

async fn list_targets(
    State(state): State<AppState>,
) -> Result<Json<Vec<TargetResponse>>, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let targets = state
        .deployment_repo
        .list_targets(ResourceId::from_uuid(tenant.id))
        .await?;

    let response: Vec<TargetResponse> = targets
        .into_iter()
        .map(|t| TargetResponse {
            id: t.id,
            name: t.name,
            target_type: t.target_type,
            region: t.region,
            status: t.status,
        })
        .collect();

    Ok(Json(response))
}

async fn create_target(
    State(state): State<AppState>,
    Json(req): Json<CreateTargetRequest>,
) -> Result<Json<TargetResponse>, ApiError> {
    let tenant = state
        .tenant_repo
        .get_by_slug("default")
        .await
        .map_err(|_| ApiError::Internal("No default tenant".to_string()))?;

    let target = state
        .deployment_repo
        .create_target(
            ResourceId::from_uuid(tenant.id),
            &req.name,
            &req.target_type,
            req.region.as_deref(),
            req.config,
        )
        .await?;

    Ok(Json(TargetResponse {
        id: target.id,
        name: target.name,
        target_type: target.target_type,
        region: target.region,
        status: target.status,
    }))
}

async fn get_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TargetResponse>, ApiError> {
    let target = state
        .deployment_repo
        .get_target(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(TargetResponse {
        id: target.id,
        name: target.name,
        target_type: target.target_type,
        region: target.region,
        status: target.status,
    }))
}

async fn delete_target(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .deployment_repo
        .delete_target(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}
