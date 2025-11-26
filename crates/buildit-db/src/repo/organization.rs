//! Organization repository - organizations, users, memberships, API keys.

use async_trait::async_trait;
use buildit_core::ResourceId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{DbError, DbResult};

/// An organization (company/account).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Organization {
    pub id: uuid::Uuid,
    pub name: String,
    pub slug: String,
    pub plan: String,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A user.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub password_hash: Option<String>,
    pub email_verified_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User without sensitive fields (for API responses).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPublic {
    pub id: uuid::Uuid,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// OAuth connection for a user.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthConnection {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_username: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub scopes: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Organization membership.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrgMembership {
    pub id: uuid::Uuid,
    pub organization_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub role: String,
    pub invited_by: Option<uuid::Uuid>,
    pub invited_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Org membership with user details.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrgMembershipWithUser {
    pub id: uuid::Uuid,
    pub organization_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub role: String,
    pub accepted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub user_name: String,
    pub user_email: String,
    pub user_avatar_url: Option<String>,
}

/// Tenant membership.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TenantMembership {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// API key (without the actual key, just metadata).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiKey {
    pub id: uuid::Uuid,
    pub organization_id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub tenant_id: Option<uuid::Uuid>,
    pub name: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Session for web auth.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: uuid::Uuid,
    pub user_id: uuid::Uuid,
    pub token_hash: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: uuid::Uuid,
    pub organization_id: Option<uuid::Uuid>,
    pub tenant_id: Option<uuid::Uuid>,
    pub user_id: Option<uuid::Uuid>,
    pub action: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<uuid::Uuid>,
    pub metadata: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait OrganizationRepo: Send + Sync {
    // Organizations
    async fn list_organizations(&self) -> DbResult<Vec<Organization>>;
    async fn get_organization(&self, id: ResourceId) -> DbResult<Organization>;
    async fn get_organization_by_slug(&self, slug: &str) -> DbResult<Organization>;
    async fn create_organization(&self, org: &Organization) -> DbResult<Organization>;
    async fn update_organization(&self, org: &Organization) -> DbResult<Organization>;

    // Users
    async fn list_users(&self) -> DbResult<Vec<UserPublic>>;
    async fn get_user(&self, id: ResourceId) -> DbResult<User>;
    async fn get_user_by_email(&self, email: &str) -> DbResult<User>;
    async fn create_user(&self, user: &User) -> DbResult<User>;
    async fn update_user(&self, user: &User) -> DbResult<User>;
    async fn update_last_login(&self, id: ResourceId) -> DbResult<()>;

    // OAuth connections
    async fn list_user_oauth_connections(
        &self,
        user_id: ResourceId,
    ) -> DbResult<Vec<OAuthConnection>>;
    async fn get_oauth_connection_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> DbResult<OAuthConnection>;

    // Org memberships
    async fn list_org_members(&self, org_id: ResourceId) -> DbResult<Vec<OrgMembershipWithUser>>;
    async fn get_org_membership(
        &self,
        org_id: ResourceId,
        user_id: ResourceId,
    ) -> DbResult<OrgMembership>;
    async fn list_user_organizations(&self, user_id: ResourceId) -> DbResult<Vec<Organization>>;
    async fn add_org_member(
        &self,
        org_id: ResourceId,
        user_id: ResourceId,
        role: &str,
        invited_by: Option<ResourceId>,
    ) -> DbResult<OrgMembership>;
    async fn update_org_member_role(
        &self,
        org_id: ResourceId,
        user_id: ResourceId,
        role: &str,
    ) -> DbResult<()>;
    async fn remove_org_member(&self, org_id: ResourceId, user_id: ResourceId) -> DbResult<()>;

    // Tenant memberships
    async fn list_tenant_members(&self, tenant_id: ResourceId) -> DbResult<Vec<TenantMembership>>;
    async fn get_tenant_membership(
        &self,
        tenant_id: ResourceId,
        user_id: ResourceId,
    ) -> DbResult<TenantMembership>;
    async fn list_user_tenants(&self, user_id: ResourceId) -> DbResult<Vec<uuid::Uuid>>;

    // API keys
    async fn list_api_keys(&self, org_id: ResourceId) -> DbResult<Vec<ApiKey>>;
    async fn get_api_key_by_prefix(&self, prefix: &str) -> DbResult<ApiKey>;
    async fn validate_api_key(&self, prefix: &str, key_hash: &str) -> DbResult<ApiKey>;
    async fn update_api_key_last_used(&self, id: ResourceId) -> DbResult<()>;

    // Sessions
    async fn create_session(&self, session: &Session) -> DbResult<Session>;
    async fn get_session_by_token(&self, token_hash: &str) -> DbResult<Session>;
    async fn delete_session(&self, id: ResourceId) -> DbResult<()>;
    async fn delete_expired_sessions(&self) -> DbResult<u64>;

    // Audit logs
    async fn create_audit_log(&self, log: &AuditLog) -> DbResult<AuditLog>;
    async fn list_audit_logs(
        &self,
        org_id: Option<ResourceId>,
        tenant_id: Option<ResourceId>,
        limit: i64,
    ) -> DbResult<Vec<AuditLog>>;
}

/// PostgreSQL implementation of OrganizationRepo.
pub struct PgOrganizationRepo {
    pool: PgPool,
}

impl PgOrganizationRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationRepo for PgOrganizationRepo {
    // Organizations
    async fn list_organizations(&self) -> DbResult<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>("SELECT * FROM organizations ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        Ok(orgs)
    }

    async fn get_organization(&self, id: ResourceId) -> DbResult<Organization> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("organization {}", id)))?;
        Ok(org)
    }

    async fn get_organization_by_slug(&self, slug: &str) -> DbResult<Organization> {
        let org = sqlx::query_as::<_, Organization>("SELECT * FROM organizations WHERE slug = $1")
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("organization with slug {}", slug)))?;
        Ok(org)
    }

    async fn create_organization(&self, org: &Organization) -> DbResult<Organization> {
        let created = sqlx::query_as::<_, Organization>(
            r#"
            INSERT INTO organizations (id, name, slug, plan, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(org.id)
        .bind(&org.name)
        .bind(&org.slug)
        .bind(&org.plan)
        .bind(&org.settings)
        .bind(org.created_at)
        .bind(org.updated_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(created)
    }

    async fn update_organization(&self, org: &Organization) -> DbResult<Organization> {
        let updated = sqlx::query_as::<_, Organization>(
            r#"
            UPDATE organizations
            SET name = $2, slug = $3, plan = $4, settings = $5, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(org.id)
        .bind(&org.name)
        .bind(&org.slug)
        .bind(&org.plan)
        .bind(&org.settings)
        .fetch_one(&self.pool)
        .await?;
        Ok(updated)
    }

    // Users
    async fn list_users(&self) -> DbResult<Vec<UserPublic>> {
        let users = sqlx::query_as::<_, UserPublic>(
            "SELECT id, email, name, avatar_url, created_at FROM users ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    async fn get_user(&self, id: ResourceId) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("user {}", id)))?;
        Ok(user)
    }

    async fn get_user_by_email(&self, email: &str) -> DbResult<User> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("user with email {}", email)))?;
        Ok(user)
    }

    async fn create_user(&self, user: &User) -> DbResult<User> {
        let created = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, email, name, avatar_url, password_hash, email_verified_at, last_login_at, settings, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.avatar_url)
        .bind(&user.password_hash)
        .bind(user.email_verified_at)
        .bind(user.last_login_at)
        .bind(&user.settings)
        .bind(user.created_at)
        .bind(user.updated_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(created)
    }

    async fn update_user(&self, user: &User) -> DbResult<User> {
        let updated = sqlx::query_as::<_, User>(
            r#"
            UPDATE users
            SET email = $2, name = $3, avatar_url = $4, password_hash = $5,
                email_verified_at = $6, settings = $7, updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(user.id)
        .bind(&user.email)
        .bind(&user.name)
        .bind(&user.avatar_url)
        .bind(&user.password_hash)
        .bind(user.email_verified_at)
        .bind(&user.settings)
        .fetch_one(&self.pool)
        .await?;
        Ok(updated)
    }

    async fn update_last_login(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // OAuth connections
    async fn list_user_oauth_connections(
        &self,
        user_id: ResourceId,
    ) -> DbResult<Vec<OAuthConnection>> {
        let connections = sqlx::query_as::<_, OAuthConnection>(
            "SELECT * FROM oauth_connections WHERE user_id = $1 ORDER BY provider",
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(connections)
    }

    async fn get_oauth_connection_by_provider(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> DbResult<OAuthConnection> {
        let connection = sqlx::query_as::<_, OAuthConnection>(
            "SELECT * FROM oauth_connections WHERE provider = $1 AND provider_user_id = $2",
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            DbError::NotFound(format!(
                "oauth connection for {} {}",
                provider, provider_user_id
            ))
        })?;
        Ok(connection)
    }

    // Org memberships
    async fn list_org_members(&self, org_id: ResourceId) -> DbResult<Vec<OrgMembershipWithUser>> {
        let members = sqlx::query_as::<_, OrgMembershipWithUser>(
            r#"
            SELECT m.id, m.organization_id, m.user_id, m.role, m.accepted_at, m.created_at,
                   u.name as user_name, u.email as user_email, u.avatar_url as user_avatar_url
            FROM org_memberships m
            JOIN users u ON m.user_id = u.id
            WHERE m.organization_id = $1
            ORDER BY u.name
            "#,
        )
        .bind(org_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(members)
    }

    async fn get_org_membership(
        &self,
        org_id: ResourceId,
        user_id: ResourceId,
    ) -> DbResult<OrgMembership> {
        let membership = sqlx::query_as::<_, OrgMembership>(
            "SELECT * FROM org_memberships WHERE organization_id = $1 AND user_id = $2",
        )
        .bind(org_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            DbError::NotFound(format!(
                "org membership for user {} in org {}",
                user_id, org_id
            ))
        })?;
        Ok(membership)
    }

    async fn list_user_organizations(&self, user_id: ResourceId) -> DbResult<Vec<Organization>> {
        let orgs = sqlx::query_as::<_, Organization>(
            r#"
            SELECT o.*
            FROM organizations o
            JOIN org_memberships m ON o.id = m.organization_id
            WHERE m.user_id = $1
            ORDER BY o.name
            "#,
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(orgs)
    }

    async fn add_org_member(
        &self,
        org_id: ResourceId,
        user_id: ResourceId,
        role: &str,
        invited_by: Option<ResourceId>,
    ) -> DbResult<OrgMembership> {
        let invited_by_uuid = invited_by.map(|id| *id.as_uuid());
        let membership = sqlx::query_as::<_, OrgMembership>(
            r#"
            INSERT INTO org_memberships (id, organization_id, user_id, role, invited_by, invited_at, accepted_at, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NULL, NOW(), NOW())
            RETURNING *
            "#
        )
        .bind(uuid::Uuid::new_v4())
        .bind(org_id.as_uuid())
        .bind(user_id.as_uuid())
        .bind(role)
        .bind(invited_by_uuid)
        .fetch_one(&self.pool)
        .await?;
        Ok(membership)
    }

    async fn update_org_member_role(
        &self,
        org_id: ResourceId,
        user_id: ResourceId,
        role: &str,
    ) -> DbResult<()> {
        sqlx::query(
            "UPDATE org_memberships SET role = $3, updated_at = NOW() WHERE organization_id = $1 AND user_id = $2"
        )
        .bind(org_id.as_uuid())
        .bind(user_id.as_uuid())
        .bind(role)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_org_member(&self, org_id: ResourceId, user_id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM org_memberships WHERE organization_id = $1 AND user_id = $2")
            .bind(org_id.as_uuid())
            .bind(user_id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Tenant memberships
    async fn list_tenant_members(&self, tenant_id: ResourceId) -> DbResult<Vec<TenantMembership>> {
        let members = sqlx::query_as::<_, TenantMembership>(
            "SELECT * FROM tenant_memberships WHERE tenant_id = $1",
        )
        .bind(tenant_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(members)
    }

    async fn get_tenant_membership(
        &self,
        tenant_id: ResourceId,
        user_id: ResourceId,
    ) -> DbResult<TenantMembership> {
        let membership = sqlx::query_as::<_, TenantMembership>(
            "SELECT * FROM tenant_memberships WHERE tenant_id = $1 AND user_id = $2",
        )
        .bind(tenant_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            DbError::NotFound(format!(
                "tenant membership for user {} in tenant {}",
                user_id, tenant_id
            ))
        })?;
        Ok(membership)
    }

    async fn list_user_tenants(&self, user_id: ResourceId) -> DbResult<Vec<uuid::Uuid>> {
        let tenants: Vec<(uuid::Uuid,)> =
            sqlx::query_as("SELECT tenant_id FROM tenant_memberships WHERE user_id = $1")
                .bind(user_id.as_uuid())
                .fetch_all(&self.pool)
                .await?;
        Ok(tenants.into_iter().map(|(id,)| id).collect())
    }

    // API keys
    async fn list_api_keys(&self, org_id: ResourceId) -> DbResult<Vec<ApiKey>> {
        let keys = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE organization_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC"
        )
        .bind(org_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        Ok(keys)
    }

    async fn get_api_key_by_prefix(&self, prefix: &str) -> DbResult<ApiKey> {
        let key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE key_prefix = $1 AND revoked_at IS NULL",
        )
        .bind(prefix)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("api key with prefix {}", prefix)))?;
        Ok(key)
    }

    async fn validate_api_key(&self, prefix: &str, key_hash: &str) -> DbResult<ApiKey> {
        let key = sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE key_prefix = $1 AND key_hash = $2 AND revoked_at IS NULL
            AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(prefix)
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("invalid or expired api key".to_string()))?;
        Ok(key)
    }

    async fn update_api_key_last_used(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Sessions
    async fn create_session(&self, session: &Session) -> DbResult<Session> {
        let created = sqlx::query_as::<_, Session>(
            r#"
            INSERT INTO sessions (id, user_id, token_hash, ip_address, user_agent, expires_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(session.id)
        .bind(session.user_id)
        .bind(&session.token_hash)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.expires_at)
        .bind(session.created_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(created)
    }

    async fn get_session_by_token(&self, token_hash: &str) -> DbResult<Session> {
        let session = sqlx::query_as::<_, Session>(
            "SELECT * FROM sessions WHERE token_hash = $1 AND expires_at > NOW()",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound("session not found or expired".to_string()))?;
        Ok(session)
    }

    async fn delete_session(&self, id: ResourceId) -> DbResult<()> {
        sqlx::query("DELETE FROM sessions WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_expired_sessions(&self) -> DbResult<u64> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < NOW()")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }

    // Audit logs
    async fn create_audit_log(&self, log: &AuditLog) -> DbResult<AuditLog> {
        let created = sqlx::query_as::<_, AuditLog>(
            r#"
            INSERT INTO audit_logs (id, organization_id, tenant_id, user_id, action, resource_type, resource_id, metadata, ip_address, user_agent, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#
        )
        .bind(log.id)
        .bind(log.organization_id)
        .bind(log.tenant_id)
        .bind(log.user_id)
        .bind(&log.action)
        .bind(&log.resource_type)
        .bind(log.resource_id)
        .bind(&log.metadata)
        .bind(&log.ip_address)
        .bind(&log.user_agent)
        .bind(log.created_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(created)
    }

    async fn list_audit_logs(
        &self,
        org_id: Option<ResourceId>,
        tenant_id: Option<ResourceId>,
        limit: i64,
    ) -> DbResult<Vec<AuditLog>> {
        let logs = match (org_id, tenant_id) {
            (Some(org), Some(tenant)) => {
                sqlx::query_as::<_, AuditLog>(
                    "SELECT * FROM audit_logs WHERE organization_id = $1 AND tenant_id = $2 ORDER BY created_at DESC LIMIT $3"
                )
                .bind(org.as_uuid())
                .bind(tenant.as_uuid())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (Some(org), None) => {
                sqlx::query_as::<_, AuditLog>(
                    "SELECT * FROM audit_logs WHERE organization_id = $1 ORDER BY created_at DESC LIMIT $2"
                )
                .bind(org.as_uuid())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, Some(tenant)) => {
                sqlx::query_as::<_, AuditLog>(
                    "SELECT * FROM audit_logs WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2"
                )
                .bind(tenant.as_uuid())
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
            (None, None) => {
                sqlx::query_as::<_, AuditLog>(
                    "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1"
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?
            }
        };
        Ok(logs)
    }
}
