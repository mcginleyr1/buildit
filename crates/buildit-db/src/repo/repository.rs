//! Repository repository (for connected Git repos).

use async_trait::async_trait;
use buildit_core::ResourceId;
use buildit_core::repository::{DetectedConfig, GitProvider, Repository, WebhookEvent};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbError, DbResult};

/// Database row for repositories.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RepositoryRow {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub provider: String,
    pub provider_id: String,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub is_private: bool,
    pub webhook_id: Option<String>,
    pub webhook_secret: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub detected_config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<RepositoryRow> for Repository {
    type Error = DbError;

    fn try_from(row: RepositoryRow) -> Result<Self, Self::Error> {
        let provider: GitProvider = row
            .provider
            .parse()
            .map_err(|e: String| DbError::InvalidData(e))?;
        let detected_config: DetectedConfig =
            serde_json::from_value(row.detected_config).unwrap_or_default();

        Ok(Repository {
            id: row.id,
            organization_id: row.organization_id,
            provider,
            provider_id: row.provider_id,
            owner: row.owner,
            name: row.name,
            full_name: row.full_name,
            clone_url: row.clone_url,
            default_branch: row.default_branch,
            is_private: row.is_private,
            webhook_id: row.webhook_id,
            webhook_secret: row.webhook_secret,
            last_synced_at: row.last_synced_at,
            detected_config,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Database row for webhook events.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebhookEventRow {
    pub id: Uuid,
    pub repository_id: Option<Uuid>,
    pub provider: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub headers: serde_json::Value,
    pub signature: Option<String>,
    pub signature_valid: Option<bool>,
    pub processed: bool,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<WebhookEventRow> for WebhookEvent {
    type Error = DbError;

    fn try_from(row: WebhookEventRow) -> Result<Self, Self::Error> {
        let provider: GitProvider = row
            .provider
            .parse()
            .map_err(|e: String| DbError::InvalidData(e))?;

        Ok(WebhookEvent {
            id: row.id,
            repository_id: row.repository_id,
            provider,
            event_type: row.event_type,
            payload: row.payload,
            headers: row.headers,
            signature: row.signature,
            signature_valid: row.signature_valid,
            processed: row.processed,
            processed_at: row.processed_at,
            error_message: row.error_message,
            created_at: row.created_at,
        })
    }
}

#[async_trait]
pub trait RepositoryRepo: Send + Sync {
    /// Create a new repository connection.
    async fn create(
        &self,
        organization_id: ResourceId,
        provider: GitProvider,
        provider_id: &str,
        owner: &str,
        name: &str,
        clone_url: &str,
        default_branch: &str,
        is_private: bool,
    ) -> DbResult<Repository>;

    /// Get a repository by ID.
    async fn get_by_id(&self, id: ResourceId) -> DbResult<Repository>;

    /// Get a repository by provider and provider ID.
    async fn get_by_provider_id(
        &self,
        provider: GitProvider,
        provider_id: &str,
    ) -> DbResult<Option<Repository>>;

    /// Get a repository by full name (owner/repo).
    async fn get_by_full_name(
        &self,
        organization_id: ResourceId,
        full_name: &str,
    ) -> DbResult<Option<Repository>>;

    /// List repositories for an organization.
    async fn list_by_organization(&self, organization_id: ResourceId) -> DbResult<Vec<Repository>>;

    /// Update detected config.
    async fn update_detected_config(
        &self,
        id: ResourceId,
        detected_config: &DetectedConfig,
    ) -> DbResult<()>;

    /// Update webhook info.
    async fn update_webhook(
        &self,
        id: ResourceId,
        webhook_id: &str,
        webhook_secret: &str,
    ) -> DbResult<()>;

    /// Update last synced timestamp.
    async fn update_last_synced(&self, id: ResourceId) -> DbResult<()>;

    /// Delete a repository.
    async fn delete(&self, id: ResourceId) -> DbResult<()>;

    /// Store a webhook event.
    async fn create_webhook_event(
        &self,
        repository_id: Option<ResourceId>,
        provider: GitProvider,
        event_type: &str,
        payload: serde_json::Value,
        headers: serde_json::Value,
        signature: Option<&str>,
    ) -> DbResult<WebhookEvent>;

    /// Mark a webhook event as processed.
    async fn mark_webhook_processed(
        &self,
        id: ResourceId,
        error_message: Option<&str>,
    ) -> DbResult<()>;

    /// Update signature validation result.
    async fn update_webhook_signature_valid(&self, id: ResourceId, valid: bool) -> DbResult<()>;
}

/// PostgreSQL implementation.
pub struct PgRepositoryRepo {
    pool: PgPool,
}

impl PgRepositoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RepositoryRepo for PgRepositoryRepo {
    async fn create(
        &self,
        organization_id: ResourceId,
        provider: GitProvider,
        provider_id: &str,
        owner: &str,
        name: &str,
        clone_url: &str,
        default_branch: &str,
        is_private: bool,
    ) -> DbResult<Repository> {
        let full_name = format!("{}/{}", owner, name);
        let row = sqlx::query_as::<_, RepositoryRow>(
            r#"
            INSERT INTO repositories (
                id, organization_id, provider, provider_id, owner, name, full_name,
                clone_url, default_branch, is_private, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(organization_id.as_uuid())
        .bind(provider.to_string())
        .bind(provider_id)
        .bind(owner)
        .bind(name)
        .bind(&full_name)
        .bind(clone_url)
        .bind(default_branch)
        .bind(is_private)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn get_by_id(&self, id: ResourceId) -> DbResult<Repository> {
        let row = sqlx::query_as::<_, RepositoryRow>("SELECT * FROM repositories WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("repository {}", id)))?;

        row.try_into()
    }

    async fn get_by_provider_id(
        &self,
        provider: GitProvider,
        provider_id: &str,
    ) -> DbResult<Option<Repository>> {
        let row = sqlx::query_as::<_, RepositoryRow>(
            "SELECT * FROM repositories WHERE provider = $1 AND provider_id = $2",
        )
        .bind(provider.to_string())
        .bind(provider_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn get_by_full_name(
        &self,
        organization_id: ResourceId,
        full_name: &str,
    ) -> DbResult<Option<Repository>> {
        let row = sqlx::query_as::<_, RepositoryRow>(
            "SELECT * FROM repositories WHERE organization_id = $1 AND full_name = $2",
        )
        .bind(organization_id.as_uuid())
        .bind(full_name)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|r| r.try_into()).transpose()
    }

    async fn list_by_organization(&self, organization_id: ResourceId) -> DbResult<Vec<Repository>> {
        let rows = sqlx::query_as::<_, RepositoryRow>(
            "SELECT * FROM repositories WHERE organization_id = $1 ORDER BY full_name",
        )
        .bind(organization_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn update_detected_config(
        &self,
        id: ResourceId,
        detected_config: &DetectedConfig,
    ) -> DbResult<()> {
        let config_json = serde_json::to_value(detected_config)
            .map_err(|e| DbError::InvalidData(e.to_string()))?;

        sqlx::query(
            "UPDATE repositories SET detected_config = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(config_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_webhook(
        &self,
        id: ResourceId,
        webhook_id: &str,
        webhook_secret: &str,
    ) -> DbResult<()> {
        sqlx::query(
            "UPDATE repositories SET webhook_id = $2, webhook_secret = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(webhook_id)
        .bind(webhook_secret)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_last_synced(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query(
            "UPDATE repositories SET last_synced_at = NOW(), updated_at = NOW() WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM repositories WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn create_webhook_event(
        &self,
        repository_id: Option<ResourceId>,
        provider: GitProvider,
        event_type: &str,
        payload: serde_json::Value,
        headers: serde_json::Value,
        signature: Option<&str>,
    ) -> DbResult<WebhookEvent> {
        let row = sqlx::query_as::<_, WebhookEventRow>(
            r#"
            INSERT INTO webhook_events (
                id, repository_id, provider, event_type, payload, headers, signature, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(repository_id.map(|r| *r.as_uuid()))
        .bind(provider.to_string())
        .bind(event_type)
        .bind(payload)
        .bind(headers)
        .bind(signature)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn mark_webhook_processed(
        &self,
        id: ResourceId,
        error_message: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query(
            "UPDATE webhook_events SET processed = true, processed_at = NOW(), error_message = $2 WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_webhook_signature_valid(&self, id: ResourceId, valid: bool) -> DbResult<()> {
        sqlx::query("UPDATE webhook_events SET signature_valid = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(valid)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
