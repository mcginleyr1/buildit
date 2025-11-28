//! Repository management endpoints.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use crate::services::git::GitService;
use buildit_core::ResourceId;
use buildit_core::repository::{DetectedConfig, GitProvider};
use buildit_db::RepositoryRepo;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_repositories).post(connect_repository))
        .route("/{id}", get(get_repository).delete(delete_repository))
        .route("/{id}/sync", post(sync_repository))
}

#[derive(Debug, Deserialize)]
pub struct ListRepositoriesQuery {
    pub organization_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct RepositoryResponse {
    pub id: Uuid,
    pub provider: String,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub is_private: bool,
    pub detected_config: DetectedConfig,
    pub last_synced_at: Option<String>,
}

async fn list_repositories(
    State(state): State<AppState>,
    Query(query): Query<ListRepositoriesQuery>,
) -> Result<Json<Vec<RepositoryResponse>>, ApiError> {
    let repos = state
        .repository_repo
        .list_by_organization(ResourceId::from_uuid(query.organization_id))
        .await?;

    let response: Vec<RepositoryResponse> = repos
        .into_iter()
        .map(|r| RepositoryResponse {
            id: r.id,
            provider: r.provider.to_string(),
            owner: r.owner,
            name: r.name,
            full_name: r.full_name,
            clone_url: r.clone_url,
            default_branch: r.default_branch,
            is_private: r.is_private,
            detected_config: r.detected_config,
            last_synced_at: r.last_synced_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct ConnectRepositoryRequest {
    pub organization_id: Uuid,
    pub provider: String,
    pub owner: String,
    pub name: String,
    /// Personal access token for cloning (required for private repos)
    pub access_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectRepositoryResponse {
    pub repository: RepositoryResponse,
    pub detected_config: DetectedConfig,
}

async fn connect_repository(
    State(state): State<AppState>,
    Json(req): Json<ConnectRepositoryRequest>,
) -> Result<Json<ConnectRepositoryResponse>, ApiError> {
    let provider: GitProvider = req
        .provider
        .parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;

    // Build clone URL based on provider
    let clone_url = match provider {
        GitProvider::Github => format!("https://github.com/{}/{}.git", req.owner, req.name),
        GitProvider::Gitlab => format!("https://gitlab.com/{}/{}.git", req.owner, req.name),
        GitProvider::Bitbucket => {
            format!("https://bitbucket.org/{}/{}.git", req.owner, req.name)
        }
    };

    // Use owner/name as provider_id for now (could fetch actual ID from API later)
    let provider_id = format!("{}/{}", req.owner, req.name);

    // Check if repository already exists
    if let Some(_existing) = state
        .repository_repo
        .get_by_full_name(
            ResourceId::from_uuid(req.organization_id),
            &format!("{}/{}", req.owner, req.name),
        )
        .await?
    {
        return Err(ApiError::BadRequest(
            "Repository already connected".to_string(),
        ));
    }

    // Create repository record
    let repo = state
        .repository_repo
        .create(
            ResourceId::from_uuid(req.organization_id),
            provider,
            &provider_id,
            &req.owner,
            &req.name,
            &clone_url,
            "main",                     // default, will be updated after clone
            req.access_token.is_some(), // assume private if token provided
        )
        .await?;

    // Clone and scan the repository
    let git_service = GitService::new();
    let detected_config = match git_service
        .clone_and_scan(&clone_url, req.access_token.as_deref())
        .await
    {
        Ok(config) => {
            // Update repository with detected config
            state
                .repository_repo
                .update_detected_config(ResourceId::from_uuid(repo.id), &config)
                .await?;
            state
                .repository_repo
                .update_last_synced(ResourceId::from_uuid(repo.id))
                .await?;
            config
        }
        Err(e) => {
            tracing::warn!(repo_id = %repo.id, error = %e, "Failed to clone and scan repository");
            // Return empty config but don't fail the connection
            DetectedConfig::default()
        }
    };

    Ok(Json(ConnectRepositoryResponse {
        repository: RepositoryResponse {
            id: repo.id,
            provider: repo.provider.to_string(),
            owner: repo.owner,
            name: repo.name,
            full_name: repo.full_name,
            clone_url: repo.clone_url,
            default_branch: repo.default_branch,
            is_private: repo.is_private,
            detected_config: detected_config.clone(),
            last_synced_at: repo.last_synced_at.map(|t| t.to_rfc3339()),
        },
        detected_config,
    }))
}

async fn get_repository(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RepositoryResponse>, ApiError> {
    let repo = state
        .repository_repo
        .get_by_id(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(RepositoryResponse {
        id: repo.id,
        provider: repo.provider.to_string(),
        owner: repo.owner,
        name: repo.name,
        full_name: repo.full_name,
        clone_url: repo.clone_url,
        default_branch: repo.default_branch,
        is_private: repo.is_private,
        detected_config: repo.detected_config,
        last_synced_at: repo.last_synced_at.map(|t| t.to_rfc3339()),
    }))
}

async fn delete_repository(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .repository_repo
        .delete(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

async fn sync_repository(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DetectedConfig>, ApiError> {
    let repo = state
        .repository_repo
        .get_by_id(ResourceId::from_uuid(id))
        .await?;

    // Re-clone and scan
    let git_service = GitService::new();
    let detected_config = git_service
        .clone_and_scan(&repo.clone_url, None) // TODO: get token from oauth_connections
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    // Update repository
    state
        .repository_repo
        .update_detected_config(ResourceId::from_uuid(id), &detected_config)
        .await?;
    state
        .repository_repo
        .update_last_synced(ResourceId::from_uuid(id))
        .await?;

    Ok(Json(detected_config))
}
