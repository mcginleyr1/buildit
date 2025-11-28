//! Pipeline repository.

use async_trait::async_trait;
use buildit_core::ResourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{DbError, DbResult};

/// A pipeline record in the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PipelineRecord {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub name: String,
    pub repository: String,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A pipeline run record.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PipelineRunRecord {
    pub id: uuid::Uuid,
    pub pipeline_id: uuid::Uuid,
    pub number: i64,
    pub status: String,
    pub trigger_info: serde_json::Value,
    pub git_info: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

/// A pipeline stage definition (template).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PipelineStageRecord {
    pub id: uuid::Uuid,
    pub pipeline_id: uuid::Uuid,
    pub name: String,
    pub image: Option<String>,
    pub commands: Vec<String>,
    pub depends_on: Vec<String>,
    pub env: serde_json::Value,
    pub timeout_seconds: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// A stage result record (run instance of a stage).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StageResultRecord {
    pub id: uuid::Uuid,
    pub pipeline_run_id: uuid::Uuid,
    pub stage_name: String,
    pub status: String,
    pub job_id: Option<uuid::Uuid>,
    pub deployment_id: Option<uuid::Uuid>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[async_trait]
pub trait PipelineRepo: Send + Sync {
    async fn create(
        &self,
        tenant_id: ResourceId,
        name: &str,
        repository: &str,
        config: serde_json::Value,
    ) -> DbResult<PipelineRecord>;
    async fn get_by_id(&self, id: ResourceId) -> DbResult<PipelineRecord>;
    async fn list_by_tenant(&self, tenant_id: ResourceId) -> DbResult<Vec<PipelineRecord>>;
    async fn update_config(
        &self,
        id: ResourceId,
        config: serde_json::Value,
    ) -> DbResult<PipelineRecord>;
    async fn delete(&self, id: ResourceId) -> DbResult<()>;

    async fn create_run(
        &self,
        pipeline_id: ResourceId,
        trigger_info: serde_json::Value,
        git_info: serde_json::Value,
    ) -> DbResult<PipelineRunRecord>;
    async fn get_run(&self, id: ResourceId) -> DbResult<PipelineRunRecord>;
    async fn list_runs(
        &self,
        pipeline_id: ResourceId,
        limit: i64,
    ) -> DbResult<Vec<PipelineRunRecord>>;
    async fn update_run_status(&self, id: ResourceId, status: &str) -> DbResult<()>;

    // Stage definition methods
    async fn list_stages(&self, pipeline_id: ResourceId) -> DbResult<Vec<PipelineStageRecord>>;
    async fn create_stage(
        &self,
        pipeline_id: ResourceId,
        name: &str,
        image: Option<&str>,
        commands: &[String],
        depends_on: &[String],
        env: serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> DbResult<PipelineStageRecord>;
    async fn delete_stages(&self, pipeline_id: ResourceId) -> DbResult<()>;

    // Stage result methods
    async fn list_stage_results(&self, run_id: ResourceId) -> DbResult<Vec<StageResultRecord>>;
    async fn create_stage_result(
        &self,
        run_id: ResourceId,
        stage_name: &str,
    ) -> DbResult<StageResultRecord>;
    async fn update_stage_result_started(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        job_id: Option<ResourceId>,
    ) -> DbResult<()>;
    async fn update_stage_result_finished(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> DbResult<()>;
}

/// PostgreSQL implementation of PipelineRepo.
pub struct PgPipelineRepo {
    pool: PgPool,
}

impl PgPipelineRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PipelineRepo for PgPipelineRepo {
    async fn create(
        &self,
        tenant_id: ResourceId,
        name: &str,
        repository: &str,
        config: serde_json::Value,
    ) -> DbResult<PipelineRecord> {
        let record = sqlx::query_as::<_, PipelineRecord>(
            r#"
            INSERT INTO pipelines (id, tenant_id, name, repository, config, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(tenant_id.as_uuid())
        .bind(name)
        .bind(repository)
        .bind(config)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    async fn get_by_id(&self, id: ResourceId) -> DbResult<PipelineRecord> {
        let record = sqlx::query_as::<_, PipelineRecord>("SELECT * FROM pipelines WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("pipeline {}", id)))?;
        Ok(record)
    }

    async fn list_by_tenant(&self, tenant_id: ResourceId) -> DbResult<Vec<PipelineRecord>> {
        let records = sqlx::query_as::<_, PipelineRecord>(
            "SELECT * FROM pipelines WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    async fn update_config(
        &self,
        id: ResourceId,
        config: serde_json::Value,
    ) -> DbResult<PipelineRecord> {
        let record = sqlx::query_as::<_, PipelineRecord>(
            r#"
            UPDATE pipelines SET config = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id.as_uuid())
        .bind(config)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("pipeline {}", id)))?;
        Ok(record)
    }

    async fn delete(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM pipelines WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_run(
        &self,
        pipeline_id: ResourceId,
        trigger_info: serde_json::Value,
        git_info: serde_json::Value,
    ) -> DbResult<PipelineRunRecord> {
        let record = sqlx::query_as::<_, PipelineRunRecord>(
            r#"
            INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info, git_info, created_at)
            VALUES ($1, $2, (SELECT COALESCE(MAX(number), 0) + 1 FROM pipeline_runs WHERE pipeline_id = $2), 'queued', $3, $4, NOW())
            RETURNING *
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(pipeline_id.as_uuid())
        .bind(trigger_info)
        .bind(git_info)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    async fn get_run(&self, id: ResourceId) -> DbResult<PipelineRunRecord> {
        let record =
            sqlx::query_as::<_, PipelineRunRecord>("SELECT * FROM pipeline_runs WHERE id = $1")
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| DbError::NotFound(format!("pipeline run {}", id)))?;
        Ok(record)
    }

    async fn list_runs(
        &self,
        pipeline_id: ResourceId,
        limit: i64,
    ) -> DbResult<Vec<PipelineRunRecord>> {
        let records = sqlx::query_as::<_, PipelineRunRecord>(
            "SELECT * FROM pipeline_runs WHERE pipeline_id = $1 ORDER BY number DESC LIMIT $2",
        )
        .bind(pipeline_id.as_uuid())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    async fn update_run_status(&self, id: ResourceId, status: &str) -> DbResult<()> {
        sqlx::query("UPDATE pipeline_runs SET status = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_stages(&self, pipeline_id: ResourceId) -> DbResult<Vec<PipelineStageRecord>> {
        let records = sqlx::query_as::<_, PipelineStageRecord>(
            "SELECT * FROM pipeline_stages WHERE pipeline_id = $1 ORDER BY created_at",
        )
        .bind(pipeline_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    async fn create_stage(
        &self,
        pipeline_id: ResourceId,
        name: &str,
        image: Option<&str>,
        commands: &[String],
        depends_on: &[String],
        env: serde_json::Value,
        timeout_seconds: Option<i32>,
    ) -> DbResult<PipelineStageRecord> {
        let record = sqlx::query_as::<_, PipelineStageRecord>(
            r#"
            INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on, env, timeout_seconds, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            RETURNING *
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(pipeline_id.as_uuid())
        .bind(name)
        .bind(image)
        .bind(commands)
        .bind(depends_on)
        .bind(env)
        .bind(timeout_seconds)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    async fn delete_stages(&self, pipeline_id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM pipeline_stages WHERE pipeline_id = $1")
            .bind(pipeline_id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_stage_results(&self, run_id: ResourceId) -> DbResult<Vec<StageResultRecord>> {
        let records = sqlx::query_as::<_, StageResultRecord>(
            "SELECT * FROM stage_results WHERE pipeline_run_id = $1 ORDER BY started_at NULLS LAST",
        )
        .bind(run_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    async fn create_stage_result(
        &self,
        run_id: ResourceId,
        stage_name: &str,
    ) -> DbResult<StageResultRecord> {
        let record = sqlx::query_as::<_, StageResultRecord>(
            r#"
            INSERT INTO stage_results (id, pipeline_run_id, stage_name, status)
            VALUES ($1, $2, $3, 'pending')
            RETURNING *
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(run_id.as_uuid())
        .bind(stage_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_stage_result_started(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        job_id: Option<ResourceId>,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE stage_results
            SET status = 'running', started_at = NOW(), job_id = $3
            WHERE pipeline_run_id = $1 AND stage_name = $2
            "#,
        )
        .bind(run_id.as_uuid())
        .bind(stage_name)
        .bind(job_id.map(|j| *j.as_uuid()))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_stage_result_finished(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        status: &str,
        error_message: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE stage_results
            SET status = $3, finished_at = NOW(), error_message = $4
            WHERE pipeline_run_id = $1 AND stage_name = $2
            "#,
        )
        .bind(run_id.as_uuid())
        .bind(stage_name)
        .bind(status)
        .bind(error_message)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
