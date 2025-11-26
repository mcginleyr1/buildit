//! Deployment repository - targets, environments, services, deployments.

use async_trait::async_trait;
use buildit_core::ResourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{DbError, DbResult};

/// A deployment target (K8s cluster, Fly.io org, etc).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Target {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub target_type: String,
    pub status: String,
    pub region: Option<String>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An environment (dev, staging, prod).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Environment {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub target_id: uuid::Uuid,
    pub name: String,
    pub health_status: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Environment with target info joined.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EnvironmentWithTarget {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub target_id: uuid::Uuid,
    pub name: String,
    pub health_status: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub target_name: String,
    pub target_type: String,
}

/// A service (deployed application).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Service {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub pipeline_id: Option<uuid::Uuid>,
    pub name: String,
    pub image: Option<String>,
    pub status: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Service with environment info.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServiceWithEnvs {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub image: Option<String>,
    pub status: String,
    pub last_deployed_at: Option<DateTime<Utc>>,
}

/// A deployment record.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Deployment {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub service_id: uuid::Uuid,
    pub environment_id: uuid::Uuid,
    pub pipeline_run_id: Option<uuid::Uuid>,
    pub version: String,
    pub commit_sha: Option<String>,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Deployment with service and environment names joined.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeploymentWithDetails {
    pub id: uuid::Uuid,
    pub version: String,
    pub commit_sha: Option<String>,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub service_name: String,
    pub environment_name: String,
}

#[async_trait]
pub trait DeploymentRepo: Send + Sync {
    // Targets
    async fn list_targets(&self, tenant_id: ResourceId) -> DbResult<Vec<Target>>;
    async fn get_target(&self, id: ResourceId) -> DbResult<Target>;

    // Environments
    async fn list_environments(
        &self,
        tenant_id: ResourceId,
    ) -> DbResult<Vec<EnvironmentWithTarget>>;
    async fn get_environment(&self, id: ResourceId) -> DbResult<Environment>;
    async fn count_services_in_environment(&self, env_id: ResourceId) -> DbResult<i64>;

    // Services
    async fn list_services(&self, tenant_id: ResourceId) -> DbResult<Vec<Service>>;
    async fn get_service(&self, id: ResourceId) -> DbResult<Service>;
    async fn get_service_environments(&self, service_id: ResourceId) -> DbResult<Vec<String>>;
    async fn get_service_last_deploy(
        &self,
        service_id: ResourceId,
    ) -> DbResult<Option<DateTime<Utc>>>;

    // Deployments
    async fn list_deployments(
        &self,
        tenant_id: ResourceId,
        limit: i64,
    ) -> DbResult<Vec<DeploymentWithDetails>>;
    async fn get_deployment(&self, id: ResourceId) -> DbResult<Deployment>;
}

/// PostgreSQL implementation of DeploymentRepo.
pub struct PgDeploymentRepo {
    pool: PgPool,
}

impl PgDeploymentRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeploymentRepo for PgDeploymentRepo {
    async fn list_targets(&self, tenant_id: ResourceId) -> DbResult<Vec<Target>> {
        let targets =
            sqlx::query_as::<_, Target>("SELECT * FROM targets WHERE tenant_id = $1 ORDER BY name")
                .bind(tenant_id.as_uuid())
                .fetch_all(&self.pool)
                .await?;
        Ok(targets)
    }

    async fn get_target(&self, id: ResourceId) -> DbResult<Target> {
        let target = sqlx::query_as::<_, Target>("SELECT * FROM targets WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("target {}", id)))?;
        Ok(target)
    }

    async fn list_environments(
        &self,
        tenant_id: ResourceId,
    ) -> DbResult<Vec<EnvironmentWithTarget>> {
        let envs = sqlx::query_as::<_, EnvironmentWithTarget>(
            r#"
            SELECT e.*, t.name as target_name, t.target_type
            FROM environments e
            JOIN targets t ON e.target_id = t.id
            WHERE e.tenant_id = $1
            ORDER BY e.name
            "#,
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(envs)
    }

    async fn get_environment(&self, id: ResourceId) -> DbResult<Environment> {
        let env = sqlx::query_as::<_, Environment>("SELECT * FROM environments WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("environment {}", id)))?;
        Ok(env)
    }

    async fn count_services_in_environment(&self, env_id: ResourceId) -> DbResult<i64> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM service_environments WHERE environment_id = $1")
                .bind(env_id.as_uuid())
                .fetch_one(&self.pool)
                .await?;
        Ok(count.0)
    }

    async fn list_services(&self, tenant_id: ResourceId) -> DbResult<Vec<Service>> {
        let services = sqlx::query_as::<_, Service>(
            "SELECT * FROM services WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(services)
    }

    async fn get_service(&self, id: ResourceId) -> DbResult<Service> {
        let service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("service {}", id)))?;
        Ok(service)
    }

    async fn get_service_environments(&self, service_id: ResourceId) -> DbResult<Vec<String>> {
        let envs: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT e.name
            FROM service_environments se
            JOIN environments e ON se.environment_id = e.id
            WHERE se.service_id = $1
            ORDER BY e.name
            "#,
        )
        .bind(service_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(envs.into_iter().map(|(name,)| name).collect())
    }

    async fn get_service_last_deploy(
        &self,
        service_id: ResourceId,
    ) -> DbResult<Option<DateTime<Utc>>> {
        let result: Option<(DateTime<Utc>,)> = sqlx::query_as(
            r#"
            SELECT MAX(last_deployed_at)
            FROM service_environments
            WHERE service_id = $1
            "#,
        )
        .bind(service_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.and_then(|(dt,)| Some(dt)))
    }

    async fn list_deployments(
        &self,
        tenant_id: ResourceId,
        limit: i64,
    ) -> DbResult<Vec<DeploymentWithDetails>> {
        let deployments = sqlx::query_as::<_, DeploymentWithDetails>(
            r#"
            SELECT d.id, d.version, d.commit_sha, d.status, d.started_at, d.finished_at, d.created_at,
                   s.name as service_name, e.name as environment_name
            FROM deployments d
            JOIN services s ON d.service_id = s.id
            JOIN environments e ON d.environment_id = e.id
            WHERE d.tenant_id = $1
            ORDER BY d.created_at DESC
            LIMIT $2
            "#
        )
        .bind(tenant_id.as_uuid())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(deployments)
    }

    async fn get_deployment(&self, id: ResourceId) -> DbResult<Deployment> {
        let deployment = sqlx::query_as::<_, Deployment>("SELECT * FROM deployments WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("deployment {}", id)))?;
        Ok(deployment)
    }
}
