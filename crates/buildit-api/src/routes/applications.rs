//! Application (GitOps) management endpoints.

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use buildit_core::ResourceId;
use buildit_core::application::{SyncPolicy, SyncTriggerType};
use buildit_db::ApplicationRepo;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_applications).post(create_application))
        .route("/{id}", get(get_application).delete(delete_application))
        .route("/{id}/syncs", get(list_syncs).post(trigger_sync))
        .route("/{id}/resources", get(list_resources))
}

#[derive(Debug, Deserialize)]
struct ListApplicationsQuery {
    tenant_id: Uuid,
}

#[derive(Debug, Serialize)]
struct ApplicationResponse {
    id: String,
    name: String,
    description: Option<String>,
    path: String,
    target_namespace: String,
    sync_policy: String,
    sync_status: String,
    health_status: String,
    synced_revision: Option<String>,
    last_synced_at: Option<String>,
    repository_id: Option<String>,
    environment_id: Option<String>,
}

async fn list_applications(
    State(state): State<AppState>,
    Query(query): Query<ListApplicationsQuery>,
) -> Result<Json<Vec<ApplicationResponse>>, ApiError> {
    let tenant_id = ResourceId::from_uuid(query.tenant_id);
    let apps = state
        .application_repo
        .list_applications_by_tenant(tenant_id)
        .await?;

    let response: Vec<ApplicationResponse> = apps
        .into_iter()
        .map(|a| ApplicationResponse {
            id: a.id.to_string(),
            name: a.name,
            description: a.description,
            path: a.path,
            target_namespace: a.target_namespace,
            sync_policy: a.sync_policy.to_string(),
            sync_status: a.sync_status.to_string(),
            health_status: a.health_status.to_string(),
            synced_revision: a.synced_revision,
            last_synced_at: a.last_synced_at.map(|t| t.to_rfc3339()),
            repository_id: a.repository_id.map(|id| id.to_string()),
            environment_id: a.environment_id.map(|id| id.to_string()),
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreateApplicationRequest {
    tenant_id: Uuid,
    name: String,
    description: Option<String>,
    repository_id: Option<Uuid>,
    environment_id: Option<Uuid>,
    path: String,
    target_namespace: String,
    sync_policy: Option<String>,
}

async fn create_application(
    State(state): State<AppState>,
    Json(req): Json<CreateApplicationRequest>,
) -> Result<Json<ApplicationResponse>, ApiError> {
    let sync_policy = match req.sync_policy.as_deref() {
        Some("auto") => SyncPolicy::Auto,
        _ => SyncPolicy::Manual,
    };

    let app = state
        .application_repo
        .create_application(
            ResourceId::from_uuid(req.tenant_id),
            &req.name,
            req.description.as_deref(),
            req.repository_id.map(ResourceId::from_uuid),
            req.environment_id.map(ResourceId::from_uuid),
            &req.path,
            &req.target_namespace,
            sync_policy,
        )
        .await?;

    Ok(Json(ApplicationResponse {
        id: app.id.to_string(),
        name: app.name,
        description: app.description,
        path: app.path,
        target_namespace: app.target_namespace,
        sync_policy: app.sync_policy.to_string(),
        sync_status: app.sync_status.to_string(),
        health_status: app.health_status.to_string(),
        synced_revision: app.synced_revision,
        last_synced_at: app.last_synced_at.map(|t| t.to_rfc3339()),
        repository_id: app.repository_id.map(|id| id.to_string()),
        environment_id: app.environment_id.map(|id| id.to_string()),
    }))
}

async fn get_application(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApplicationResponse>, ApiError> {
    let app = state
        .application_repo
        .get_application(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(ApplicationResponse {
        id: app.id.to_string(),
        name: app.name,
        description: app.description,
        path: app.path,
        target_namespace: app.target_namespace,
        sync_policy: app.sync_policy.to_string(),
        sync_status: app.sync_status.to_string(),
        health_status: app.health_status.to_string(),
        synced_revision: app.synced_revision,
        last_synced_at: app.last_synced_at.map(|t| t.to_rfc3339()),
        repository_id: app.repository_id.map(|id| id.to_string()),
        environment_id: app.environment_id.map(|id| id.to_string()),
    }))
}

async fn delete_application(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), ApiError> {
    state
        .application_repo
        .delete_application(ResourceId::from_uuid(id))
        .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct SyncResponse {
    id: String,
    application_id: String,
    revision: String,
    status: String,
    trigger_type: String,
    resources_created: i32,
    resources_updated: i32,
    resources_deleted: i32,
    error_message: Option<String>,
    started_at: Option<String>,
    finished_at: Option<String>,
    created_at: String,
}

async fn list_syncs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<SyncResponse>>, ApiError> {
    let syncs = state
        .application_repo
        .list_syncs(ResourceId::from_uuid(id), 20)
        .await?;

    let response: Vec<SyncResponse> = syncs
        .into_iter()
        .map(|s| SyncResponse {
            id: s.id.to_string(),
            application_id: s.application_id.to_string(),
            revision: s.revision,
            status: s.status.to_string(),
            trigger_type: s.trigger_type.to_string(),
            resources_created: s.resources_created,
            resources_updated: s.resources_updated,
            resources_deleted: s.resources_deleted,
            error_message: s.error_message,
            started_at: s.started_at.map(|t| t.to_rfc3339()),
            finished_at: s.finished_at.map(|t| t.to_rfc3339()),
            created_at: s.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct TriggerSyncRequest {
    revision: Option<String>,
}

async fn trigger_sync(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TriggerSyncRequest>,
) -> Result<Json<SyncResponse>, ApiError> {
    // Get the application to find the repository
    let app = state
        .application_repo
        .get_application(ResourceId::from_uuid(id))
        .await?;

    // Determine revision - use provided or get latest from repo
    let revision = req.revision.unwrap_or_else(|| "HEAD".to_string());

    // Create sync record
    let sync = state
        .application_repo
        .create_sync(
            ResourceId::from_uuid(app.id),
            &revision,
            None, // TODO: Get current user
            SyncTriggerType::Manual,
        )
        .await?;

    // TODO: Actually perform the sync in background
    // For now, just return the sync record
    // In a real implementation, this would:
    // 1. Clone/fetch the repository
    // 2. Parse manifests from app.path
    // 3. Apply to target cluster
    // 4. Update sync status

    Ok(Json(SyncResponse {
        id: sync.id.to_string(),
        application_id: sync.application_id.to_string(),
        revision: sync.revision,
        status: sync.status.to_string(),
        trigger_type: sync.trigger_type.to_string(),
        resources_created: sync.resources_created,
        resources_updated: sync.resources_updated,
        resources_deleted: sync.resources_deleted,
        error_message: sync.error_message,
        started_at: sync.started_at.map(|t| t.to_rfc3339()),
        finished_at: sync.finished_at.map(|t| t.to_rfc3339()),
        created_at: sync.created_at.to_rfc3339(),
    }))
}

#[derive(Debug, Serialize)]
struct ResourceResponse {
    id: String,
    api_version: String,
    kind: String,
    name: String,
    namespace: String,
    status: String,
    health_status: String,
    out_of_sync: bool,
}

async fn list_resources(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ResourceResponse>>, ApiError> {
    let resources = state
        .application_repo
        .list_resources(ResourceId::from_uuid(id))
        .await?;

    let response: Vec<ResourceResponse> = resources
        .into_iter()
        .map(|r| ResourceResponse {
            id: r.id.to_string(),
            api_version: r.api_version,
            kind: r.kind,
            name: r.name,
            namespace: r.namespace,
            status: r.status.to_string(),
            health_status: r.health_status.to_string(),
            out_of_sync: r.out_of_sync,
        })
        .collect();

    Ok(Json(response))
}
