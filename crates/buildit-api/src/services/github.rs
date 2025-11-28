//! GitHub API client for OAuth and repository operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GitHub OAuth configuration.
#[derive(Debug, Clone)]
pub struct GitHubConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl GitHubConfig {
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("GITHUB_CLIENT_ID").ok()?;
        let client_secret = std::env::var("GITHUB_CLIENT_SECRET").ok()?;
        let redirect_uri = std::env::var("GITHUB_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:30080/auth/github/callback".to_string());

        Some(Self {
            client_id,
            client_secret,
            redirect_uri,
        })
    }

    /// Generate the OAuth authorization URL.
    pub fn auth_url(&self, state: &str) -> String {
        let scopes = "repo,read:user";
        format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
            self.client_id,
            urlencoding::encode(&self.redirect_uri),
            scopes,
            state
        )
    }
}

/// GitHub API client.
pub struct GitHubClient {
    client: reqwest::Client,
    access_token: String,
}

impl GitHubClient {
    pub fn new(access_token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            access_token,
        }
    }

    /// Exchange an OAuth code for an access token.
    pub async fn exchange_code(
        config: &GitHubConfig,
        code: &str,
    ) -> Result<TokenResponse, GitHubError> {
        let client = reqwest::Client::new();

        let params = [
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", config.redirect_uri.as_str()),
        ];

        let response = client
            .post("https://github.com/login/oauth/access_token")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| GitHubError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api(format!("Token exchange failed: {}", text)));
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| GitHubError::Parse(e.to_string()))?;

        if token.access_token.is_empty() {
            return Err(GitHubError::Api("No access token in response".to_string()));
        }

        Ok(token)
    }

    /// Get the authenticated user's information.
    pub async fn get_user(&self) -> Result<GitHubUser, GitHubError> {
        let response = self
            .client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("User-Agent", "BuildIt-CI")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| GitHubError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api(format!("Failed to get user: {}", text)));
        }

        response
            .json()
            .await
            .map_err(|e| GitHubError::Parse(e.to_string()))
    }

    /// List repositories the user has access to.
    pub async fn list_repos(
        &self,
        page: u32,
        per_page: u32,
    ) -> Result<Vec<GitHubRepo>, GitHubError> {
        let url = format!(
            "https://api.github.com/user/repos?sort=updated&direction=desc&page={}&per_page={}&type=all",
            page, per_page
        );

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("User-Agent", "BuildIt-CI")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| GitHubError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api(format!("Failed to list repos: {}", text)));
        }

        response
            .json()
            .await
            .map_err(|e| GitHubError::Parse(e.to_string()))
    }

    /// Search repositories by name.
    pub async fn search_repos(&self, query: &str) -> Result<Vec<GitHubRepo>, GitHubError> {
        // First get user's repos and filter
        let repos = self.list_repos(1, 100).await?;
        let query_lower = query.to_lowercase();

        Ok(repos
            .into_iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&query_lower)
                    || r.full_name.to_lowercase().contains(&query_lower)
            })
            .collect())
    }

    /// Get a specific repository by owner and name.
    pub async fn get_repo(&self, owner: &str, repo: &str) -> Result<GitHubRepo, GitHubError> {
        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("User-Agent", "BuildIt-CI")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| GitHubError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api(format!(
                "Failed to get repo ({}): {}",
                status, text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| GitHubError::Parse(e.to_string()))
    }

    /// Create a webhook on a repository.
    pub async fn create_webhook(
        &self,
        owner: &str,
        repo: &str,
        webhook_url: &str,
        secret: &str,
    ) -> Result<WebhookResponse, GitHubError> {
        let url = format!("https://api.github.com/repos/{}/{}/hooks", owner, repo);

        let payload = serde_json::json!({
            "name": "web",
            "active": true,
            "events": ["push", "pull_request"],
            "config": {
                "url": webhook_url,
                "content_type": "json",
                "secret": secret,
                "insecure_ssl": "0"
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
            .header("User-Agent", "BuildIt-CI")
            .header("Accept", "application/vnd.github+json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| GitHubError::Request(e.to_string()))?;

        if !response.status().is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(GitHubError::Api(format!(
                "Failed to create webhook: {}",
                text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| GitHubError::Parse(e.to_string()))
    }
}

/// OAuth token response.
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

/// GitHub user information.
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: String,
}

/// GitHub repository information.
#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub owner: RepoOwner,
    pub private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub default_branch: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RepoOwner {
    pub login: String,
    pub id: i64,
    pub avatar_url: String,
}

/// Webhook creation response.
#[derive(Debug, Deserialize)]
pub struct WebhookResponse {
    pub id: i64,
    pub active: bool,
    pub events: Vec<String>,
    pub config: WebhookConfig,
}

#[derive(Debug, Deserialize)]
pub struct WebhookConfig {
    pub url: String,
    pub content_type: String,
}

/// GitHub API errors.
#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("Request failed: {0}")]
    Request(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Parse error: {0}")]
    Parse(String),
}
