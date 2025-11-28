//! Application repository (GitOps).

use async_trait::async_trait;
use buildit_core::ResourceId;
use buildit_core::application::{
    Application, ApplicationResource, ApplicationSync, ApplicationSyncStatus, HealthStatus,
    ResourceStatus, SyncPolicy, SyncStatus, SyncTriggerType,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbError, DbResult};

/// Database row for applications.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApplicationRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub repository_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub target_namespace: String,
    pub target_cluster: Option<String>,
    pub sync_policy: String,
    pub prune: bool,
    pub self_heal: bool,
    pub sync_status: String,
    pub health_status: String,
    pub synced_revision: Option<String>,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ApplicationRow> for Application {
    type Error = DbError;

    fn try_from(row: ApplicationRow) -> Result<Self, Self::Error> {
        let sync_policy = match row.sync_policy.as_str() {
            "auto" => SyncPolicy::Auto,
            _ => SyncPolicy::Manual,
        };

        let sync_status = match row.sync_status.as_str() {
            "synced" => SyncStatus::Synced,
            "out_of_sync" => SyncStatus::OutOfSync,
            "syncing" => SyncStatus::Syncing,
            _ => SyncStatus::Unknown,
        };

        let health_status = match row.health_status.as_str() {
            "healthy" => HealthStatus::Healthy,
            "progressing" => HealthStatus::Progressing,
            "degraded" => HealthStatus::Degraded,
            "suspended" => HealthStatus::Suspended,
            "missing" => HealthStatus::Missing,
            _ => HealthStatus::Unknown,
        };

        Ok(Application {
            id: row.id,
            tenant_id: row.tenant_id,
            repository_id: row.repository_id,
            environment_id: row.environment_id,
            name: row.name,
            description: row.description,
            path: row.path,
            target_namespace: row.target_namespace,
            target_cluster: row.target_cluster,
            sync_policy,
            prune: row.prune,
            self_heal: row.self_heal,
            sync_status,
            health_status,
            synced_revision: row.synced_revision,
            last_synced_at: row.last_synced_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Database row for application syncs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApplicationSyncRow {
    pub id: Uuid,
    pub application_id: Uuid,
    pub revision: String,
    pub status: String,
    pub triggered_by: Option<Uuid>,
    pub trigger_type: String,
    pub resources_created: i32,
    pub resources_updated: i32,
    pub resources_deleted: i32,
    pub error_message: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<ApplicationSyncRow> for ApplicationSync {
    type Error = DbError;

    fn try_from(row: ApplicationSyncRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "running" => ApplicationSyncStatus::Running,
            "succeeded" => ApplicationSyncStatus::Succeeded,
            "failed" => ApplicationSyncStatus::Failed,
            _ => ApplicationSyncStatus::Pending,
        };

        let trigger_type = match row.trigger_type.as_str() {
            "webhook" => SyncTriggerType::Webhook,
            "auto" => SyncTriggerType::Auto,
            "scheduled" => SyncTriggerType::Scheduled,
            _ => SyncTriggerType::Manual,
        };

        Ok(ApplicationSync {
            id: row.id,
            application_id: row.application_id,
            revision: row.revision,
            status,
            triggered_by: row.triggered_by,
            trigger_type,
            resources_created: row.resources_created,
            resources_updated: row.resources_updated,
            resources_deleted: row.resources_deleted,
            error_message: row.error_message,
            started_at: row.started_at,
            finished_at: row.finished_at,
            created_at: row.created_at,
        })
    }
}

/// Database row for application resources.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApplicationResourceRow {
    pub id: Uuid,
    pub application_id: Uuid,
    pub api_group: String,
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub health_status: String,
    pub out_of_sync: bool,
    pub desired_state: Option<serde_json::Value>,
    pub live_state: Option<serde_json::Value>,
    pub diff: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ApplicationResourceRow> for ApplicationResource {
    type Error = DbError;

    fn try_from(row: ApplicationResourceRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "synced" => ResourceStatus::Synced,
            "out_of_sync" => ResourceStatus::OutOfSync,
            "missing" => ResourceStatus::Missing,
            "orphaned" => ResourceStatus::Orphaned,
            _ => ResourceStatus::Unknown,
        };

        let health_status = match row.health_status.as_str() {
            "healthy" => HealthStatus::Healthy,
            "progressing" => HealthStatus::Progressing,
            "degraded" => HealthStatus::Degraded,
            "suspended" => HealthStatus::Suspended,
            "missing" => HealthStatus::Missing,
            _ => HealthStatus::Unknown,
        };

        Ok(ApplicationResource {
            id: row.id,
            application_id: row.application_id,
            api_group: row.api_group,
            api_version: row.api_version,
            kind: row.kind,
            name: row.name,
            namespace: row.namespace,
            status,
            health_status,
            out_of_sync: row.out_of_sync,
            desired_state: row.desired_state,
            live_state: row.live_state,
            diff: row.diff,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait]
pub trait ApplicationRepo: Send + Sync {
    // Application CRUD
    async fn create_application(
        &self,
        tenant_id: ResourceId,
        name: &str,
        description: Option<&str>,
        repository_id: Option<ResourceId>,
        environment_id: Option<ResourceId>,
        path: &str,
        target_namespace: &str,
        sync_policy: SyncPolicy,
    ) -> DbResult<Application>;

    async fn get_application(&self, id: ResourceId) -> DbResult<Application>;
    async fn list_applications_by_tenant(
        &self,
        tenant_id: ResourceId,
    ) -> DbResult<Vec<Application>>;
    async fn list_applications_by_repository(
        &self,
        repository_id: ResourceId,
    ) -> DbResult<Vec<Application>>;
    async fn update_application_sync_status(
        &self,
        id: ResourceId,
        sync_status: SyncStatus,
        health_status: HealthStatus,
        synced_revision: Option<&str>,
    ) -> DbResult<()>;
    async fn delete_application(&self, id: ResourceId) -> DbResult<()>;

    // Application syncs
    async fn create_sync(
        &self,
        application_id: ResourceId,
        revision: &str,
        triggered_by: Option<ResourceId>,
        trigger_type: SyncTriggerType,
    ) -> DbResult<ApplicationSync>;
    async fn get_sync(&self, id: ResourceId) -> DbResult<ApplicationSync>;
    async fn list_syncs(
        &self,
        application_id: ResourceId,
        limit: i64,
    ) -> DbResult<Vec<ApplicationSync>>;
    async fn update_sync_started(&self, id: ResourceId) -> DbResult<()>;
    async fn update_sync_finished(
        &self,
        id: ResourceId,
        status: ApplicationSyncStatus,
        resources_created: i32,
        resources_updated: i32,
        resources_deleted: i32,
        error_message: Option<&str>,
    ) -> DbResult<()>;

    // Application resources
    async fn upsert_resource(
        &self,
        application_id: ResourceId,
        api_group: &str,
        api_version: &str,
        kind: &str,
        name: &str,
        namespace: &str,
        status: ResourceStatus,
        health_status: HealthStatus,
        out_of_sync: bool,
        desired_state: Option<serde_json::Value>,
        live_state: Option<serde_json::Value>,
        diff: Option<&str>,
    ) -> DbResult<ApplicationResource>;
    async fn list_resources(
        &self,
        application_id: ResourceId,
    ) -> DbResult<Vec<ApplicationResource>>;
    async fn delete_orphaned_resources(
        &self,
        application_id: ResourceId,
        keep_names: &[(String, String, String)], // (kind, name, namespace)
    ) -> DbResult<i64>;
}

/// PostgreSQL implementation.
pub struct PgApplicationRepo {
    pool: PgPool,
}

impl PgApplicationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ApplicationRepo for PgApplicationRepo {
    async fn create_application(
        &self,
        tenant_id: ResourceId,
        name: &str,
        description: Option<&str>,
        repository_id: Option<ResourceId>,
        environment_id: Option<ResourceId>,
        path: &str,
        target_namespace: &str,
        sync_policy: SyncPolicy,
    ) -> DbResult<Application> {
        let row = sqlx::query_as::<_, ApplicationRow>(
            r#"
            INSERT INTO applications (
                id, tenant_id, repository_id, environment_id, name, description,
                path, target_namespace, sync_policy, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id.as_uuid())
        .bind(repository_id.map(|r| *r.as_uuid()))
        .bind(environment_id.map(|e| *e.as_uuid()))
        .bind(name)
        .bind(description)
        .bind(path)
        .bind(target_namespace)
        .bind(sync_policy.to_string())
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn get_application(&self, id: ResourceId) -> DbResult<Application> {
        let row = sqlx::query_as::<_, ApplicationRow>("SELECT * FROM applications WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("application {}", id)))?;

        row.try_into()
    }

    async fn list_applications_by_tenant(
        &self,
        tenant_id: ResourceId,
    ) -> DbResult<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            "SELECT * FROM applications WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn list_applications_by_repository(
        &self,
        repository_id: ResourceId,
    ) -> DbResult<Vec<Application>> {
        let rows = sqlx::query_as::<_, ApplicationRow>(
            "SELECT * FROM applications WHERE repository_id = $1 ORDER BY name",
        )
        .bind(repository_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn update_application_sync_status(
        &self,
        id: ResourceId,
        sync_status: SyncStatus,
        health_status: HealthStatus,
        synced_revision: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE applications SET
                sync_status = $2,
                health_status = $3,
                synced_revision = COALESCE($4, synced_revision),
                last_synced_at = CASE WHEN $4 IS NOT NULL THEN NOW() ELSE last_synced_at END,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(sync_status.to_string())
        .bind(health_status.to_string())
        .bind(synced_revision)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_application(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM applications WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn create_sync(
        &self,
        application_id: ResourceId,
        revision: &str,
        triggered_by: Option<ResourceId>,
        trigger_type: SyncTriggerType,
    ) -> DbResult<ApplicationSync> {
        let row = sqlx::query_as::<_, ApplicationSyncRow>(
            r#"
            INSERT INTO application_syncs (
                id, application_id, revision, triggered_by, trigger_type, created_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(application_id.as_uuid())
        .bind(revision)
        .bind(triggered_by.map(|u| *u.as_uuid()))
        .bind(trigger_type.to_string())
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn get_sync(&self, id: ResourceId) -> DbResult<ApplicationSync> {
        let row = sqlx::query_as::<_, ApplicationSyncRow>(
            "SELECT * FROM application_syncs WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("application sync {}", id)))?;

        row.try_into()
    }

    async fn list_syncs(
        &self,
        application_id: ResourceId,
        limit: i64,
    ) -> DbResult<Vec<ApplicationSync>> {
        let rows = sqlx::query_as::<_, ApplicationSyncRow>(
            "SELECT * FROM application_syncs WHERE application_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(application_id.as_uuid())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn update_sync_started(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query(
            "UPDATE application_syncs SET status = 'running', started_at = NOW() WHERE id = $1",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_sync_finished(
        &self,
        id: ResourceId,
        status: ApplicationSyncStatus,
        resources_created: i32,
        resources_updated: i32,
        resources_deleted: i32,
        error_message: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE application_syncs SET
                status = $2,
                resources_created = $3,
                resources_updated = $4,
                resources_deleted = $5,
                error_message = $6,
                finished_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(status.to_string())
        .bind(resources_created)
        .bind(resources_updated)
        .bind(resources_deleted)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn upsert_resource(
        &self,
        application_id: ResourceId,
        api_group: &str,
        api_version: &str,
        kind: &str,
        name: &str,
        namespace: &str,
        status: ResourceStatus,
        health_status: HealthStatus,
        out_of_sync: bool,
        desired_state: Option<serde_json::Value>,
        live_state: Option<serde_json::Value>,
        diff: Option<&str>,
    ) -> DbResult<ApplicationResource> {
        let row = sqlx::query_as::<_, ApplicationResourceRow>(
            r#"
            INSERT INTO application_resources (
                id, application_id, api_group, api_version, kind, name, namespace,
                status, health_status, out_of_sync, desired_state, live_state, diff,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, NOW(), NOW())
            ON CONFLICT (application_id, api_group, kind, name, namespace) DO UPDATE SET
                api_version = EXCLUDED.api_version,
                status = EXCLUDED.status,
                health_status = EXCLUDED.health_status,
                out_of_sync = EXCLUDED.out_of_sync,
                desired_state = EXCLUDED.desired_state,
                live_state = EXCLUDED.live_state,
                diff = EXCLUDED.diff,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(application_id.as_uuid())
        .bind(api_group)
        .bind(api_version)
        .bind(kind)
        .bind(name)
        .bind(namespace)
        .bind(status.to_string())
        .bind(health_status.to_string())
        .bind(out_of_sync)
        .bind(desired_state)
        .bind(live_state)
        .bind(diff)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn list_resources(
        &self,
        application_id: ResourceId,
    ) -> DbResult<Vec<ApplicationResource>> {
        let rows = sqlx::query_as::<_, ApplicationResourceRow>(
            "SELECT * FROM application_resources WHERE application_id = $1 ORDER BY kind, name",
        )
        .bind(application_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn delete_orphaned_resources(
        &self,
        application_id: ResourceId,
        keep_names: &[(String, String, String)], // (kind, name, namespace)
    ) -> DbResult<i64> {
        // Build a list of resources to keep
        // This is a bit tricky - we need to delete resources that aren't in the keep list
        if keep_names.is_empty() {
            // Delete all resources for this application
            let result = sqlx::query("DELETE FROM application_resources WHERE application_id = $1")
                .bind(application_id.as_uuid())
                .execute(&self.pool)
                .await?;
            return Ok(result.rows_affected() as i64);
        }

        // For each resource to keep, we build a condition
        // This is inefficient but works for now
        let mut deleted = 0i64;
        let existing = self.list_resources(application_id).await?;
        for resource in existing {
            let key = (
                resource.kind.clone(),
                resource.name.clone(),
                resource.namespace.clone(),
            );
            if !keep_names.contains(&key) {
                sqlx::query("DELETE FROM application_resources WHERE id = $1")
                    .bind(resource.id)
                    .execute(&self.pool)
                    .await?;
                deleted += 1;
            }
        }

        Ok(deleted)
    }
}
