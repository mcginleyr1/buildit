//! Tenant repository.

use async_trait::async_trait;
use buildit_core::ResourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{DbError, DbResult};

/// A tenant in the system.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tenant {
    pub id: uuid::Uuid,
    pub name: String,
    pub slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait TenantRepo: Send + Sync {
    async fn create(&self, name: &str, slug: &str) -> DbResult<Tenant>;
    async fn get_by_id(&self, id: ResourceId) -> DbResult<Tenant>;
    async fn get_by_slug(&self, slug: &str) -> DbResult<Tenant>;
    async fn list(&self) -> DbResult<Vec<Tenant>>;
    async fn delete(&self, id: ResourceId) -> DbResult<()>;
}

/// PostgreSQL implementation of TenantRepo.
pub struct PgTenantRepo {
    pool: PgPool,
}

impl PgTenantRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TenantRepo for PgTenantRepo {
    async fn create(&self, name: &str, slug: &str) -> DbResult<Tenant> {
        let tenant = sqlx::query_as::<_, Tenant>(
            r#"
            INSERT INTO tenants (id, name, slug, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(name)
        .bind(slug)
        .fetch_one(&self.pool)
        .await?;
        Ok(tenant)
    }

    async fn get_by_id(&self, id: ResourceId) -> DbResult<Tenant> {
        let tenant = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("tenant {}", id)))?;
        Ok(tenant)
    }

    async fn get_by_slug(&self, slug: &str) -> DbResult<Tenant> {
        let tenant = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("tenant with slug {}", slug)))?;
        Ok(tenant)
    }

    async fn list(&self) -> DbResult<Vec<Tenant>> {
        let tenants = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(tenants)
    }

    async fn delete(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
