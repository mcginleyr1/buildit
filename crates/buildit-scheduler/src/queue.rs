//! Job queue implementation using PostgreSQL.

use buildit_core::ResourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// A queued job.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QueuedJob {
    pub id: uuid::Uuid,
    pub pipeline_run_id: uuid::Uuid,
    pub stage_name: String,
    pub priority: i32,
    pub status: String,
    pub claimed_by: Option<String>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Job queue backed by PostgreSQL.
pub struct JobQueue {
    pool: PgPool,
}

impl JobQueue {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Enqueue a new job.
    pub async fn enqueue(
        &self,
        pipeline_run_id: ResourceId,
        stage_name: &str,
        priority: i32,
    ) -> Result<QueuedJob, sqlx::Error> {
        let job = sqlx::query_as::<_, QueuedJob>(
            r#"
            INSERT INTO job_queue (id, pipeline_run_id, stage_name, priority, status, created_at)
            VALUES ($1, $2, $3, $4, 'pending', NOW())
            RETURNING *
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(pipeline_run_id.as_uuid())
        .bind(stage_name)
        .bind(priority)
        .fetch_one(&self.pool)
        .await?;
        Ok(job)
    }

    /// Claim the next available job.
    /// Uses SKIP LOCKED to prevent contention in distributed environments.
    pub async fn claim(&self, worker_id: &str) -> Result<Option<QueuedJob>, sqlx::Error> {
        let job = sqlx::query_as::<_, QueuedJob>(
            r#"
            UPDATE job_queue
            SET status = 'claimed', claimed_by = $1, claimed_at = NOW()
            WHERE id = (
                SELECT id FROM job_queue
                WHERE status = 'pending'
                ORDER BY priority DESC, created_at ASC
                FOR UPDATE SKIP LOCKED
                LIMIT 1
            )
            RETURNING *
            "#,
        )
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(job)
    }

    /// Mark a job as completed.
    pub async fn complete(&self, job_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE job_queue SET status = 'completed' WHERE id = $1")
            .bind(job_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark a job as failed.
    pub async fn fail(&self, job_id: uuid::Uuid, error: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE job_queue SET status = 'failed', error = $2 WHERE id = $1")
            .bind(job_id)
            .bind(error)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Release a claimed job back to pending (e.g., on worker crash recovery).
    pub async fn release(&self, job_id: uuid::Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE job_queue SET status = 'pending', claimed_by = NULL, claimed_at = NULL WHERE id = $1"
        )
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
