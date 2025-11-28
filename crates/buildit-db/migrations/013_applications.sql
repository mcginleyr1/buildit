-- Applications table (GitOps deployments)
CREATE TABLE IF NOT EXISTS applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    repository_id UUID REFERENCES repositories(id) ON DELETE SET NULL,
    environment_id UUID REFERENCES environments(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    path VARCHAR(1024) NOT NULL DEFAULT '.',
    target_namespace VARCHAR(255) NOT NULL DEFAULT 'default',
    target_cluster VARCHAR(255),
    sync_policy VARCHAR(50) NOT NULL DEFAULT 'manual',
    prune BOOLEAN NOT NULL DEFAULT false,
    self_heal BOOLEAN NOT NULL DEFAULT false,
    sync_status VARCHAR(50) NOT NULL DEFAULT 'unknown',
    health_status VARCHAR(50) NOT NULL DEFAULT 'unknown',
    synced_revision VARCHAR(255),
    last_synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX IF NOT EXISTS idx_applications_tenant ON applications(tenant_id);
CREATE INDEX IF NOT EXISTS idx_applications_repository ON applications(repository_id);
CREATE INDEX IF NOT EXISTS idx_applications_environment ON applications(environment_id);

-- Application syncs table (sync history)
CREATE TABLE IF NOT EXISTS application_syncs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    revision VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    triggered_by UUID REFERENCES users(id) ON DELETE SET NULL,
    trigger_type VARCHAR(50) NOT NULL DEFAULT 'manual',
    resources_created INT NOT NULL DEFAULT 0,
    resources_updated INT NOT NULL DEFAULT 0,
    resources_deleted INT NOT NULL DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_application_syncs_app ON application_syncs(application_id);
CREATE INDEX IF NOT EXISTS idx_application_syncs_created ON application_syncs(created_at DESC);

-- Application resources table (tracked K8s resources)
CREATE TABLE IF NOT EXISTS application_resources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
    api_group VARCHAR(255) NOT NULL DEFAULT '',
    api_version VARCHAR(50) NOT NULL,
    kind VARCHAR(100) NOT NULL,
    name VARCHAR(255) NOT NULL,
    namespace VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'unknown',
    health_status VARCHAR(50) NOT NULL DEFAULT 'unknown',
    out_of_sync BOOLEAN NOT NULL DEFAULT false,
    desired_state JSONB,
    live_state JSONB,
    diff TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(application_id, api_group, kind, name, namespace)
);

CREATE INDEX IF NOT EXISTS idx_application_resources_app ON application_resources(application_id);
