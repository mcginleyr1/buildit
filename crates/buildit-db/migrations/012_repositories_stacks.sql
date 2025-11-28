-- Connected repositories (GitHub, GitLab, etc.)
CREATE TABLE repositories (
    id UUID PRIMARY KEY,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL, -- 'github', 'gitlab', 'bitbucket'
    provider_id VARCHAR(255) NOT NULL, -- Provider's repo ID
    owner VARCHAR(255) NOT NULL, -- Owner/org name on provider
    name VARCHAR(255) NOT NULL, -- Repo name
    full_name VARCHAR(512) NOT NULL, -- e.g., 'owner/repo'
    clone_url VARCHAR(512) NOT NULL, -- HTTPS clone URL
    default_branch VARCHAR(255) NOT NULL DEFAULT 'main',
    is_private BOOLEAN NOT NULL DEFAULT false,
    webhook_id VARCHAR(255), -- ID of webhook we created on provider
    webhook_secret VARCHAR(255), -- Secret for validating webhook payloads
    last_synced_at TIMESTAMPTZ,
    detected_config JSONB NOT NULL DEFAULT '{}', -- Detected .buildit.kdl, .tf files, etc.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(organization_id, provider, provider_id)
);

CREATE INDEX idx_repositories_org ON repositories(organization_id);
CREATE INDEX idx_repositories_provider ON repositories(provider, provider_id);
CREATE INDEX idx_repositories_full_name ON repositories(full_name);

-- Stacks (Terraform workspaces)
CREATE TABLE stacks (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    repository_id UUID REFERENCES repositories(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    path VARCHAR(512) NOT NULL DEFAULT '.', -- Path within repo to terraform files
    terraform_version VARCHAR(50) NOT NULL DEFAULT '1.5.0',
    auto_apply BOOLEAN NOT NULL DEFAULT false, -- Auto-apply after successful plan
    working_directory VARCHAR(512), -- Local clone path
    var_file VARCHAR(512), -- Path to .tfvars file
    backend_config JSONB NOT NULL DEFAULT '{}', -- Backend configuration overrides
    environment_variables JSONB NOT NULL DEFAULT '{}', -- TF_VAR_* and others
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'initializing', 'ready', 'error'
    last_run_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(tenant_id, name)
);

CREATE INDEX idx_stacks_tenant ON stacks(tenant_id);
CREATE INDEX idx_stacks_repository ON stacks(repository_id);
CREATE INDEX idx_stacks_status ON stacks(status);

-- Stack variables (sensitive and non-sensitive)
CREATE TABLE stack_variables (
    id UUID PRIMARY KEY,
    stack_id UUID NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value TEXT, -- NULL if sensitive (stored encrypted elsewhere or in secrets manager)
    is_sensitive BOOLEAN NOT NULL DEFAULT false,
    is_hcl BOOLEAN NOT NULL DEFAULT false, -- If true, value is HCL not string
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(stack_id, key)
);

CREATE INDEX idx_stack_variables_stack ON stack_variables(stack_id);

-- Stack runs (plan/apply operations)
CREATE TABLE stack_runs (
    id UUID PRIMARY KEY,
    stack_id UUID NOT NULL REFERENCES stacks(id) ON DELETE CASCADE,
    run_type VARCHAR(50) NOT NULL, -- 'plan', 'apply', 'destroy', 'refresh'
    status VARCHAR(50) NOT NULL DEFAULT 'pending', -- 'pending', 'running', 'needs_approval', 'approved', 'applying', 'succeeded', 'failed', 'cancelled'
    triggered_by UUID REFERENCES users(id) ON DELETE SET NULL,
    trigger_type VARCHAR(50) NOT NULL DEFAULT 'manual', -- 'manual', 'webhook', 'drift', 'scheduled'
    commit_sha VARCHAR(40),
    plan_output TEXT, -- Terraform plan output
    plan_json JSONB, -- Parsed plan for UI display
    apply_output TEXT, -- Terraform apply output
    resources_to_add INTEGER DEFAULT 0,
    resources_to_change INTEGER DEFAULT 0,
    resources_to_destroy INTEGER DEFAULT 0,
    approved_by UUID REFERENCES users(id) ON DELETE SET NULL,
    approved_at TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stack_runs_stack ON stack_runs(stack_id);
CREATE INDEX idx_stack_runs_status ON stack_runs(status);
CREATE INDEX idx_stack_runs_created ON stack_runs(created_at DESC);

-- Stack state (Terraform state storage - alternative to S3/remote backend)
CREATE TABLE stack_state (
    id UUID PRIMARY KEY,
    stack_id UUID NOT NULL REFERENCES stacks(id) ON DELETE CASCADE UNIQUE,
    state_json JSONB NOT NULL,
    serial INTEGER NOT NULL DEFAULT 0,
    lineage VARCHAR(255),
    lock_id VARCHAR(255),
    locked_by UUID REFERENCES users(id) ON DELETE SET NULL,
    locked_at TIMESTAMPTZ,
    lock_info JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_stack_state_stack ON stack_state(stack_id);
CREATE INDEX idx_stack_state_lock ON stack_state(lock_id);

-- Link stacks to environments they provision
ALTER TABLE environments ADD COLUMN stack_id UUID REFERENCES stacks(id) ON DELETE SET NULL;
ALTER TABLE environments ADD COLUMN stack_outputs JSONB NOT NULL DEFAULT '{}';
CREATE INDEX idx_environments_stack ON environments(stack_id);

-- Link pipelines to repositories
ALTER TABLE pipelines ADD COLUMN repository_id UUID REFERENCES repositories(id) ON DELETE SET NULL;
CREATE INDEX idx_pipelines_repository ON pipelines(repository_id);

-- Webhooks received (for debugging/history)
CREATE TABLE webhook_events (
    id UUID PRIMARY KEY,
    repository_id UUID REFERENCES repositories(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL,
    event_type VARCHAR(100) NOT NULL, -- 'push', 'pull_request', 'tag', etc.
    payload JSONB NOT NULL,
    headers JSONB NOT NULL DEFAULT '{}',
    signature VARCHAR(255),
    signature_valid BOOLEAN,
    processed BOOLEAN NOT NULL DEFAULT false,
    processed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_webhook_events_repo ON webhook_events(repository_id);
CREATE INDEX idx_webhook_events_created ON webhook_events(created_at DESC);
CREATE INDEX idx_webhook_events_processed ON webhook_events(processed) WHERE NOT processed;
