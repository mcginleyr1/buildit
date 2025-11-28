//! Authentication routes (GitHub OAuth, etc.)

use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use axum::{Json, Router};
use axum_extra::extract::CookieJar;
use axum_extra::extract::cookie::Cookie;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::error::ApiError;
use crate::services::github::{GitHubClient, GitHubConfig, GitHubRepo};

/// Cookie name for storing GitHub access token.
const GITHUB_TOKEN_COOKIE: &str = "github_token";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/github", get(github_auth))
        .route("/github/callback", get(github_callback))
        .route("/github/repos", get(list_github_repos))
        .route("/github/repos/search", get(search_github_repos))
        .route("/github/status", get(github_status))
        .route("/github/disconnect", get(github_disconnect))
}

/// Redirect to GitHub OAuth.
async fn github_auth() -> Result<Response, ApiError> {
    let config = GitHubConfig::from_env().ok_or_else(|| {
        ApiError::Internal(
            "GitHub OAuth not configured. Set GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET"
                .to_string(),
        )
    })?;

    // Generate a random state for CSRF protection
    let state = Uuid::new_v4().to_string();

    // TODO: Store state in session for verification

    let auth_url = config.auth_url(&state);
    Ok(Redirect::temporary(&auth_url).into_response())
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// Handle GitHub OAuth callback.
async fn github_callback(
    State(_state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<(CookieJar, Redirect), ApiError> {
    let config = GitHubConfig::from_env()
        .ok_or_else(|| ApiError::Internal("GitHub OAuth not configured".to_string()))?;

    // Exchange code for token
    let token = GitHubClient::exchange_code(&config, &query.code)
        .await
        .map_err(|e| ApiError::Internal(format!("OAuth exchange failed: {}", e)))?;

    // Get user info
    let client = GitHubClient::new(token.access_token.clone());
    let user = client
        .get_user()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to get user: {}", e)))?;

    tracing::info!(
        user = %user.login,
        "GitHub OAuth successful"
    );

    // Store token in a cookie (in production, use HTTP-only secure cookie)
    let cookie = Cookie::build((GITHUB_TOKEN_COOKIE, token.access_token))
        .path("/")
        .max_age(time::Duration::days(30))
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build();

    let jar = jar.add(cookie);

    // Redirect back to the pipeline creation page
    Ok((jar, Redirect::to("/pipelines/new?github_connected=true")))
}

#[derive(Debug, Deserialize)]
pub struct ReposQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct RepoResponse {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: String,
    pub private: bool,
    pub clone_url: String,
    pub default_branch: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub updated_at: String,
}

impl From<GitHubRepo> for RepoResponse {
    fn from(repo: GitHubRepo) -> Self {
        Self {
            id: repo.id,
            name: repo.name,
            full_name: repo.full_name,
            owner: repo.owner.login,
            private: repo.private,
            clone_url: repo.clone_url,
            default_branch: repo.default_branch,
            description: repo.description,
            language: repo.language,
            updated_at: repo.updated_at,
        }
    }
}

/// Extract GitHub token from cookie.
fn get_github_token(jar: &CookieJar) -> Result<String, ApiError> {
    jar.get(GITHUB_TOKEN_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| {
            ApiError::Unauthorized(
                "GitHub not connected. Please connect your GitHub account first.".to_string(),
            )
        })
}

/// List repositories for the authenticated user.
async fn list_github_repos(
    jar: CookieJar,
    Query(query): Query<ReposQuery>,
) -> Result<Json<Vec<RepoResponse>>, ApiError> {
    let token = get_github_token(&jar)?;

    let client = GitHubClient::new(token);
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(30).min(100);

    let repos = client
        .list_repos(page, per_page)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list repos: {}", e)))?;

    Ok(Json(repos.into_iter().map(RepoResponse::from).collect()))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// Search repositories by name.
async fn search_github_repos(
    jar: CookieJar,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<RepoResponse>>, ApiError> {
    let token = get_github_token(&jar)?;

    let client = GitHubClient::new(token);

    let repos = client
        .search_repos(&query.q)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to search repos: {}", e)))?;

    Ok(Json(repos.into_iter().map(RepoResponse::from).collect()))
}

#[derive(Debug, Serialize)]
pub struct GitHubStatusResponse {
    pub configured: bool,
    pub connected: bool,
    pub user: Option<String>,
}

/// Check if GitHub OAuth is configured and if user is connected.
async fn github_status(jar: CookieJar) -> Result<Json<GitHubStatusResponse>, ApiError> {
    let config = GitHubConfig::from_env();
    let configured = config.is_some();

    // Check if user has a valid token
    let (connected, user) = if let Some(cookie) = jar.get(GITHUB_TOKEN_COOKIE) {
        let client = GitHubClient::new(cookie.value().to_string());
        match client.get_user().await {
            Ok(u) => (true, Some(u.login)),
            Err(_) => (false, None), // Token expired or invalid
        }
    } else {
        (false, None)
    };

    Ok(Json(GitHubStatusResponse {
        configured,
        connected,
        user,
    }))
}

/// Disconnect GitHub (remove token cookie).
async fn github_disconnect(jar: CookieJar) -> (CookieJar, Redirect) {
    let jar = jar.remove(Cookie::from(GITHUB_TOKEN_COOKIE));
    (jar, Redirect::to("/pipelines/new"))
}
