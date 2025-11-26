-- Deployment targets (K8s clusters, Fly.io orgs, Cloud Run projects)
CREATE TABLE targets (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    target_type VARCHAR(50) NOT NULL, -- 'kubernetes', 'fly', 'cloudrun'
    status VARCHAR(50) NOT NULL DEFAULT 'connected',
    region VARCHAR(100),
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX idx_targets_tenant ON targets(tenant_id);

-- Environments (dev, staging, prod) linked to targets
CREATE TABLE environments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    health_status VARCHAR(50) NOT NULL DEFAULT 'unknown', -- 'healthy', 'degraded', 'unhealthy', 'unknown'
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX idx_environments_tenant ON environments(tenant_id);
CREATE INDEX idx_environments_target ON environments(target_id);

-- Services (deployed applications)
CREATE TABLE services (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    pipeline_id UUID REFERENCES pipelines(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    image VARCHAR(512),
    status VARCHAR(50) NOT NULL DEFAULT 'unknown', -- 'healthy', 'degraded', 'unhealthy', 'unknown'
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX idx_services_tenant ON services(tenant_id);
CREATE INDEX idx_services_pipeline ON services(pipeline_id);

-- Service-environment mapping (which services are deployed to which environments)
CREATE TABLE service_environments (
    id UUID PRIMARY KEY,
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    current_version VARCHAR(100),
    status VARCHAR(50) NOT NULL DEFAULT 'unknown',
    last_deployed_at TIMESTAMPTZ,
    config JSONB NOT NULL DEFAULT '{}',
    UNIQUE(service_id, environment_id)
);

CREATE INDEX idx_service_environments_service ON service_environments(service_id);
CREATE INDEX idx_service_environments_environment ON service_environments(environment_id);

-- Deployment history
CREATE TABLE deployments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    service_id UUID NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    environment_id UUID NOT NULL REFERENCES environments(id) ON DELETE CASCADE,
    pipeline_run_id UUID REFERENCES pipeline_runs(id) ON DELETE SET NULL,
    version VARCHAR(100) NOT NULL,
    commit_sha VARCHAR(40),
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'running', 'succeeded', 'failed', 'cancelled'
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    config JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_deployments_tenant ON deployments(tenant_id);
CREATE INDEX idx_deployments_service ON deployments(service_id);
CREATE INDEX idx_deployments_environment ON deployments(environment_id);
CREATE INDEX idx_deployments_status ON deployments(status);
CREATE INDEX idx_deployments_created ON deployments(created_at DESC);
