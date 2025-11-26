//! Tenant management endpoints.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::AppState;
use crate::error::ApiError;
use buildit_db::TenantRepo;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tenants).post(create_tenant))
        .route("/{slug}", get(get_tenant))
}

#[derive(Debug, Serialize)]
struct TenantResponse {
    id: String,
    name: String,
    slug: String,
}

async fn list_tenants(
    State(state): State<AppState>,
) -> Result<Json<Vec<TenantResponse>>, ApiError> {
    let tenants = state.tenant_repo.list().await?;
    let response: Vec<TenantResponse> = tenants
        .into_iter()
        .map(|t| TenantResponse {
            id: t.id.to_string(),
            name: t.name,
            slug: t.slug,
        })
        .collect();
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct CreateTenantRequest {
    name: String,
    slug: String,
}

async fn create_tenant(
    State(state): State<AppState>,
    Json(req): Json<CreateTenantRequest>,
) -> Result<Json<TenantResponse>, ApiError> {
    let tenant = state.tenant_repo.create(&req.name, &req.slug).await?;
    Ok(Json(TenantResponse {
        id: tenant.id.to_string(),
        name: tenant.name,
        slug: tenant.slug,
    }))
}

async fn get_tenant(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<TenantResponse>, ApiError> {
    let tenant = state.tenant_repo.get_by_slug(&slug).await?;
    Ok(Json(TenantResponse {
        id: tenant.id.to_string(),
        name: tenant.name,
        slug: tenant.slug,
    }))
}
