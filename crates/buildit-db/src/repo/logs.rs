//! Log repository for storing and retrieving pipeline execution logs.

use async_trait::async_trait;
use buildit_core::ResourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::DbResult;

/// A log entry record from the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LogRecord {
    pub id: uuid::Uuid,
    pub pipeline_run_id: uuid::Uuid,
    pub stage_name: String,
    pub timestamp: DateTime<Utc>,
    pub stream: String,
    pub content: String,
}

#[async_trait]
pub trait LogRepo: Send + Sync {
    /// Append a log line for a stage.
    async fn append_log(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        stream: &str,
        content: &str,
    ) -> DbResult<()>;

    /// Append multiple log lines at once (batch insert).
    async fn append_logs_batch(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        logs: &[(String, String)], // (stream, content)
    ) -> DbResult<()>;

    /// Get all logs for a run.
    async fn get_logs_for_run(&self, run_id: ResourceId) -> DbResult<Vec<LogRecord>>;

    /// Get logs for a specific stage in a run.
    async fn get_logs_for_stage(
        &self,
        run_id: ResourceId,
        stage_name: &str,
    ) -> DbResult<Vec<LogRecord>>;

    /// Get logs with pagination (offset-based).
    async fn get_logs_paginated(
        &self,
        run_id: ResourceId,
        stage_name: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> DbResult<Vec<LogRecord>>;
}

/// PostgreSQL implementation of LogRepo.
pub struct PgLogRepo {
    pool: PgPool,
}

impl PgLogRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LogRepo for PgLogRepo {
    async fn append_log(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        stream: &str,
        content: &str,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO logs (id, pipeline_run_id, stage_name, stream, content, timestamp)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
        )
        .bind(uuid::Uuid::now_v7())
        .bind(run_id.as_uuid())
        .bind(stage_name)
        .bind(stream)
        .bind(content)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn append_logs_batch(
        &self,
        run_id: ResourceId,
        stage_name: &str,
        logs: &[(String, String)],
    ) -> DbResult<()> {
        if logs.is_empty() {
            return Ok(());
        }

        // Build batch insert
        let mut query_builder = sqlx::QueryBuilder::new(
            "INSERT INTO logs (id, pipeline_run_id, stage_name, stream, content, timestamp) ",
        );

        query_builder.push_values(logs.iter(), |mut b, (stream, content)| {
            b.push_bind(uuid::Uuid::now_v7())
                .push_bind(run_id.as_uuid())
                .push_bind(stage_name)
                .push_bind(stream)
                .push_bind(content)
                .push("NOW()");
        });

        let query = query_builder.build();
        query.execute(&self.pool).await?;
        Ok(())
    }

    async fn get_logs_for_run(&self, run_id: ResourceId) -> DbResult<Vec<LogRecord>> {
        let records = sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT id, pipeline_run_id, stage_name, timestamp, stream, content
            FROM logs
            WHERE pipeline_run_id = $1
            ORDER BY timestamp ASC
            "#,
        )
        .bind(run_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    async fn get_logs_for_stage(
        &self,
        run_id: ResourceId,
        stage_name: &str,
    ) -> DbResult<Vec<LogRecord>> {
        let records = sqlx::query_as::<_, LogRecord>(
            r#"
            SELECT id, pipeline_run_id, stage_name, timestamp, stream, content
            FROM logs
            WHERE pipeline_run_id = $1 AND stage_name = $2
            ORDER BY timestamp ASC
            "#,
        )
        .bind(run_id.as_uuid())
        .bind(stage_name)
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    async fn get_logs_paginated(
        &self,
        run_id: ResourceId,
        stage_name: Option<&str>,
        offset: i64,
        limit: i64,
    ) -> DbResult<Vec<LogRecord>> {
        let records = if let Some(stage) = stage_name {
            sqlx::query_as::<_, LogRecord>(
                r#"
                SELECT id, pipeline_run_id, stage_name, timestamp, stream, content
                FROM logs
                WHERE pipeline_run_id = $1 AND stage_name = $2
                ORDER BY timestamp ASC
                OFFSET $3 LIMIT $4
                "#,
            )
            .bind(run_id.as_uuid())
            .bind(stage)
            .bind(offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, LogRecord>(
                r#"
                SELECT id, pipeline_run_id, stage_name, timestamp, stream, content
                FROM logs
                WHERE pipeline_run_id = $1
                ORDER BY timestamp ASC
                OFFSET $2 LIMIT $3
                "#,
            )
            .bind(run_id.as_uuid())
            .bind(offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(records)
    }
}
