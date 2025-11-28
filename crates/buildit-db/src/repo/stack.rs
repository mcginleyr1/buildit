//! Stack repository (for Terraform/IaC).

use async_trait::async_trait;
use buildit_core::ResourceId;
use buildit_core::stack::{
    Stack, StackRun, StackRunStatus, StackRunType, StackState, StackStatus, StackTriggerType,
    StackVariable,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{DbError, DbResult};

/// Database row for stacks.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StackRow {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub repository_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub terraform_version: String,
    pub auto_apply: bool,
    pub working_directory: Option<String>,
    pub var_file: Option<String>,
    pub backend_config: serde_json::Value,
    pub environment_variables: serde_json::Value,
    pub status: String,
    pub last_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<StackRow> for Stack {
    type Error = DbError;

    fn try_from(row: StackRow) -> Result<Self, Self::Error> {
        let status = match row.status.as_str() {
            "pending" => StackStatus::Pending,
            "initializing" => StackStatus::Initializing,
            "ready" => StackStatus::Ready,
            "error" => StackStatus::Error,
            _ => StackStatus::Pending,
        };

        Ok(Stack {
            id: row.id,
            tenant_id: row.tenant_id,
            repository_id: row.repository_id,
            name: row.name,
            description: row.description,
            path: row.path,
            terraform_version: row.terraform_version,
            auto_apply: row.auto_apply,
            working_directory: row.working_directory,
            var_file: row.var_file,
            backend_config: row.backend_config,
            environment_variables: row.environment_variables,
            status,
            last_run_at: row.last_run_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

/// Database row for stack variables.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StackVariableRow {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub key: String,
    pub value: Option<String>,
    pub is_sensitive: bool,
    pub is_hcl: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<StackVariableRow> for StackVariable {
    fn from(row: StackVariableRow) -> Self {
        StackVariable {
            id: row.id,
            stack_id: row.stack_id,
            key: row.key,
            value: row.value,
            is_sensitive: row.is_sensitive,
            is_hcl: row.is_hcl,
            description: row.description,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database row for stack runs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StackRunRow {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub run_type: String,
    pub status: String,
    pub triggered_by: Option<Uuid>,
    pub trigger_type: String,
    pub commit_sha: Option<String>,
    pub plan_output: Option<String>,
    pub plan_json: Option<serde_json::Value>,
    pub apply_output: Option<String>,
    pub resources_to_add: Option<i32>,
    pub resources_to_change: Option<i32>,
    pub resources_to_destroy: Option<i32>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<StackRunRow> for StackRun {
    type Error = DbError;

    fn try_from(row: StackRunRow) -> Result<Self, Self::Error> {
        let run_type = match row.run_type.as_str() {
            "plan" => StackRunType::Plan,
            "apply" => StackRunType::Apply,
            "destroy" => StackRunType::Destroy,
            "refresh" => StackRunType::Refresh,
            _ => {
                return Err(DbError::InvalidData(format!(
                    "Unknown run type: {}",
                    row.run_type
                )));
            }
        };

        let status = match row.status.as_str() {
            "pending" => StackRunStatus::Pending,
            "running" => StackRunStatus::Running,
            "needs_approval" => StackRunStatus::NeedsApproval,
            "approved" => StackRunStatus::Approved,
            "applying" => StackRunStatus::Applying,
            "succeeded" => StackRunStatus::Succeeded,
            "failed" => StackRunStatus::Failed,
            "cancelled" => StackRunStatus::Cancelled,
            _ => StackRunStatus::Pending,
        };

        let trigger_type = match row.trigger_type.as_str() {
            "manual" => StackTriggerType::Manual,
            "webhook" => StackTriggerType::Webhook,
            "drift" => StackTriggerType::Drift,
            "scheduled" => StackTriggerType::Scheduled,
            _ => StackTriggerType::Manual,
        };

        Ok(StackRun {
            id: row.id,
            stack_id: row.stack_id,
            run_type,
            status,
            triggered_by: row.triggered_by,
            trigger_type,
            commit_sha: row.commit_sha,
            plan_output: row.plan_output,
            plan_json: row.plan_json,
            apply_output: row.apply_output,
            resources_to_add: row.resources_to_add.unwrap_or(0),
            resources_to_change: row.resources_to_change.unwrap_or(0),
            resources_to_destroy: row.resources_to_destroy.unwrap_or(0),
            approved_by: row.approved_by,
            approved_at: row.approved_at,
            started_at: row.started_at,
            finished_at: row.finished_at,
            error_message: row.error_message,
            created_at: row.created_at,
        })
    }
}

/// Database row for stack state.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StackStateRow {
    pub id: Uuid,
    pub stack_id: Uuid,
    pub state_json: serde_json::Value,
    pub serial: i32,
    pub lineage: Option<String>,
    pub lock_id: Option<String>,
    pub locked_by: Option<Uuid>,
    pub locked_at: Option<DateTime<Utc>>,
    pub lock_info: Option<serde_json::Value>,
    pub updated_at: DateTime<Utc>,
}

impl From<StackStateRow> for StackState {
    fn from(row: StackStateRow) -> Self {
        StackState {
            id: row.id,
            stack_id: row.stack_id,
            state_json: row.state_json,
            serial: row.serial,
            lineage: row.lineage,
            lock_id: row.lock_id,
            locked_by: row.locked_by,
            locked_at: row.locked_at,
            lock_info: row.lock_info,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
pub trait StackRepo: Send + Sync {
    // Stack CRUD
    async fn create_stack(
        &self,
        tenant_id: ResourceId,
        name: &str,
        description: Option<&str>,
        repository_id: Option<ResourceId>,
        path: &str,
        terraform_version: &str,
        auto_apply: bool,
    ) -> DbResult<Stack>;

    async fn get_stack(&self, id: ResourceId) -> DbResult<Stack>;
    async fn list_stacks_by_tenant(&self, tenant_id: ResourceId) -> DbResult<Vec<Stack>>;
    async fn list_stacks_by_repository(&self, repository_id: ResourceId) -> DbResult<Vec<Stack>>;
    async fn update_stack_status(&self, id: ResourceId, status: StackStatus) -> DbResult<()>;
    async fn update_stack_working_directory(&self, id: ResourceId, dir: &str) -> DbResult<()>;
    async fn delete_stack(&self, id: ResourceId) -> DbResult<()>;

    // Stack variables
    async fn list_variables(&self, stack_id: ResourceId) -> DbResult<Vec<StackVariable>>;
    async fn set_variable(
        &self,
        stack_id: ResourceId,
        key: &str,
        value: Option<&str>,
        is_sensitive: bool,
        is_hcl: bool,
        description: Option<&str>,
    ) -> DbResult<StackVariable>;
    async fn delete_variable(&self, stack_id: ResourceId, key: &str) -> DbResult<()>;

    // Stack runs
    async fn create_run(
        &self,
        stack_id: ResourceId,
        run_type: StackRunType,
        triggered_by: Option<ResourceId>,
        trigger_type: StackTriggerType,
        commit_sha: Option<&str>,
    ) -> DbResult<StackRun>;

    async fn get_run(&self, id: ResourceId) -> DbResult<StackRun>;
    async fn list_runs(&self, stack_id: ResourceId, limit: i64) -> DbResult<Vec<StackRun>>;
    async fn update_run_status(&self, id: ResourceId, status: StackRunStatus) -> DbResult<()>;
    async fn update_run_started(&self, id: ResourceId) -> DbResult<()>;
    async fn update_run_plan_output(
        &self,
        id: ResourceId,
        output: &str,
        plan_json: Option<serde_json::Value>,
        to_add: i32,
        to_change: i32,
        to_destroy: i32,
    ) -> DbResult<()>;
    async fn update_run_apply_output(&self, id: ResourceId, output: &str) -> DbResult<()>;
    async fn update_run_finished(
        &self,
        id: ResourceId,
        status: StackRunStatus,
        error_message: Option<&str>,
    ) -> DbResult<()>;
    async fn approve_run(&self, id: ResourceId, user_id: ResourceId) -> DbResult<()>;

    // Stack state
    async fn get_state(&self, stack_id: ResourceId) -> DbResult<Option<StackState>>;
    async fn save_state(
        &self,
        stack_id: ResourceId,
        state_json: serde_json::Value,
        serial: i32,
        lineage: Option<&str>,
    ) -> DbResult<StackState>;
    async fn lock_state(
        &self,
        stack_id: ResourceId,
        lock_id: &str,
        user_id: ResourceId,
        lock_info: serde_json::Value,
    ) -> DbResult<bool>;
    async fn unlock_state(&self, stack_id: ResourceId, lock_id: &str) -> DbResult<bool>;
}

/// PostgreSQL implementation.
pub struct PgStackRepo {
    pool: PgPool,
}

impl PgStackRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StackRepo for PgStackRepo {
    async fn create_stack(
        &self,
        tenant_id: ResourceId,
        name: &str,
        description: Option<&str>,
        repository_id: Option<ResourceId>,
        path: &str,
        terraform_version: &str,
        auto_apply: bool,
    ) -> DbResult<Stack> {
        let row = sqlx::query_as::<_, StackRow>(
            r#"
            INSERT INTO stacks (
                id, tenant_id, repository_id, name, description, path,
                terraform_version, auto_apply, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id.as_uuid())
        .bind(repository_id.map(|r| *r.as_uuid()))
        .bind(name)
        .bind(description)
        .bind(path)
        .bind(terraform_version)
        .bind(auto_apply)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn get_stack(&self, id: ResourceId) -> DbResult<Stack> {
        let row = sqlx::query_as::<_, StackRow>("SELECT * FROM stacks WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("stack {}", id)))?;

        row.try_into()
    }

    async fn list_stacks_by_tenant(&self, tenant_id: ResourceId) -> DbResult<Vec<Stack>> {
        let rows = sqlx::query_as::<_, StackRow>(
            "SELECT * FROM stacks WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn list_stacks_by_repository(&self, repository_id: ResourceId) -> DbResult<Vec<Stack>> {
        let rows = sqlx::query_as::<_, StackRow>(
            "SELECT * FROM stacks WHERE repository_id = $1 ORDER BY name",
        )
        .bind(repository_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn update_stack_status(&self, id: ResourceId, status: StackStatus) -> DbResult<()> {
        sqlx::query("UPDATE stacks SET status = $2, updated_at = NOW() WHERE id = $1")
            .bind(id.as_uuid())
            .bind(status.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_stack_working_directory(&self, id: ResourceId, dir: &str) -> DbResult<()> {
        sqlx::query("UPDATE stacks SET working_directory = $2, updated_at = NOW() WHERE id = $1")
            .bind(id.as_uuid())
            .bind(dir)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_stack(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM stacks WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn list_variables(&self, stack_id: ResourceId) -> DbResult<Vec<StackVariable>> {
        let rows = sqlx::query_as::<_, StackVariableRow>(
            "SELECT * FROM stack_variables WHERE stack_id = $1 ORDER BY key",
        )
        .bind(stack_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn set_variable(
        &self,
        stack_id: ResourceId,
        key: &str,
        value: Option<&str>,
        is_sensitive: bool,
        is_hcl: bool,
        description: Option<&str>,
    ) -> DbResult<StackVariable> {
        let row = sqlx::query_as::<_, StackVariableRow>(
            r#"
            INSERT INTO stack_variables (id, stack_id, key, value, is_sensitive, is_hcl, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            ON CONFLICT (stack_id, key) DO UPDATE SET
                value = EXCLUDED.value,
                is_sensitive = EXCLUDED.is_sensitive,
                is_hcl = EXCLUDED.is_hcl,
                description = EXCLUDED.description,
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(stack_id.as_uuid())
        .bind(key)
        .bind(value)
        .bind(is_sensitive)
        .bind(is_hcl)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn delete_variable(&self, stack_id: ResourceId, key: &str) -> DbResult<()> {
        sqlx::query("DELETE FROM stack_variables WHERE stack_id = $1 AND key = $2")
            .bind(stack_id.as_uuid())
            .bind(key)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn create_run(
        &self,
        stack_id: ResourceId,
        run_type: StackRunType,
        triggered_by: Option<ResourceId>,
        trigger_type: StackTriggerType,
        commit_sha: Option<&str>,
    ) -> DbResult<StackRun> {
        let run_type_str = match run_type {
            StackRunType::Plan => "plan",
            StackRunType::Apply => "apply",
            StackRunType::Destroy => "destroy",
            StackRunType::Refresh => "refresh",
        };

        let row = sqlx::query_as::<_, StackRunRow>(
            r#"
            INSERT INTO stack_runs (
                id, stack_id, run_type, status, triggered_by, trigger_type, commit_sha, created_at
            )
            VALUES ($1, $2, $3, 'pending', $4, $5, $6, NOW())
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(stack_id.as_uuid())
        .bind(run_type_str)
        .bind(triggered_by.map(|u| *u.as_uuid()))
        .bind(trigger_type.to_string())
        .bind(commit_sha)
        .fetch_one(&self.pool)
        .await?;

        row.try_into()
    }

    async fn get_run(&self, id: ResourceId) -> DbResult<StackRun> {
        let row = sqlx::query_as::<_, StackRunRow>("SELECT * FROM stack_runs WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("stack run {}", id)))?;

        row.try_into()
    }

    async fn list_runs(&self, stack_id: ResourceId, limit: i64) -> DbResult<Vec<StackRun>> {
        let rows = sqlx::query_as::<_, StackRunRow>(
            "SELECT * FROM stack_runs WHERE stack_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(stack_id.as_uuid())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|r| r.try_into()).collect()
    }

    async fn update_run_status(&self, id: ResourceId, status: StackRunStatus) -> DbResult<()> {
        sqlx::query("UPDATE stack_runs SET status = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(status.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_run_started(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("UPDATE stack_runs SET status = 'running', started_at = NOW() WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_run_plan_output(
        &self,
        id: ResourceId,
        output: &str,
        plan_json: Option<serde_json::Value>,
        to_add: i32,
        to_change: i32,
        to_destroy: i32,
    ) -> DbResult<()> {
        sqlx::query(
            r#"
            UPDATE stack_runs SET
                plan_output = $2,
                plan_json = $3,
                resources_to_add = $4,
                resources_to_change = $5,
                resources_to_destroy = $6
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(output)
        .bind(plan_json)
        .bind(to_add)
        .bind(to_change)
        .bind(to_destroy)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_run_apply_output(&self, id: ResourceId, output: &str) -> DbResult<()> {
        sqlx::query("UPDATE stack_runs SET apply_output = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(output)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_run_finished(
        &self,
        id: ResourceId,
        status: StackRunStatus,
        error_message: Option<&str>,
    ) -> DbResult<()> {
        sqlx::query(
            "UPDATE stack_runs SET status = $2, finished_at = NOW(), error_message = $3 WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(status.to_string())
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        // Update last_run_at on the stack
        sqlx::query(
            "UPDATE stacks SET last_run_at = NOW() WHERE id = (SELECT stack_id FROM stack_runs WHERE id = $1)",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn approve_run(&self, id: ResourceId, user_id: ResourceId) -> DbResult<()> {
        sqlx::query(
            "UPDATE stack_runs SET status = 'approved', approved_by = $2, approved_at = NOW() WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(user_id.as_uuid())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_state(&self, stack_id: ResourceId) -> DbResult<Option<StackState>> {
        let row =
            sqlx::query_as::<_, StackStateRow>("SELECT * FROM stack_state WHERE stack_id = $1")
                .bind(stack_id.as_uuid())
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn save_state(
        &self,
        stack_id: ResourceId,
        state_json: serde_json::Value,
        serial: i32,
        lineage: Option<&str>,
    ) -> DbResult<StackState> {
        let row = sqlx::query_as::<_, StackStateRow>(
            r#"
            INSERT INTO stack_state (id, stack_id, state_json, serial, lineage, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (stack_id) DO UPDATE SET
                state_json = EXCLUDED.state_json,
                serial = EXCLUDED.serial,
                lineage = COALESCE(EXCLUDED.lineage, stack_state.lineage),
                updated_at = NOW()
            RETURNING *
            "#,
        )
        .bind(Uuid::now_v7())
        .bind(stack_id.as_uuid())
        .bind(state_json)
        .bind(serial)
        .bind(lineage)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn lock_state(
        &self,
        stack_id: ResourceId,
        lock_id: &str,
        user_id: ResourceId,
        lock_info: serde_json::Value,
    ) -> DbResult<bool> {
        // Try to acquire lock only if not already locked
        let result = sqlx::query(
            r#"
            UPDATE stack_state SET
                lock_id = $2,
                locked_by = $3,
                locked_at = NOW(),
                lock_info = $4
            WHERE stack_id = $1 AND lock_id IS NULL
            "#,
        )
        .bind(stack_id.as_uuid())
        .bind(lock_id)
        .bind(user_id.as_uuid())
        .bind(lock_info)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn unlock_state(&self, stack_id: ResourceId, lock_id: &str) -> DbResult<bool> {
        let result = sqlx::query(
            r#"
            UPDATE stack_state SET
                lock_id = NULL,
                locked_by = NULL,
                locked_at = NULL,
                lock_info = NULL
            WHERE stack_id = $1 AND lock_id = $2
            "#,
        )
        .bind(stack_id.as_uuid())
        .bind(lock_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
