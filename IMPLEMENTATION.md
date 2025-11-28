# BuildIt Implementation Status

Current implementation status and recent changes.

---

## Recently Completed (2024-11-28)

### Stage Result Persistence ✅
- Added `create_stage_result`, `update_stage_result_started`, `update_stage_result_finished` to PipelineRepo
- Wired up orchestrator events to persist stage results in real-time
- Stage results now stored with actual timestamps and status
- Run duration calculated from earliest stage start to latest stage finish

### Variable Interpolation System ✅
- Created `buildit-config/src/variables.rs` with `VariableContext` and `VariableContextBuilder`
- Supports: `${git.sha}`, `${git.branch}`, `${git.short_sha}`, `${git.message}`, `${git.author}`
- Supports: `${pipeline.id}`, `${pipeline.name}`, `${run.id}`, `${run.number}`
- Supports: `${stage.name}`, `${stage.index}`, `${env.VAR}`, `${secrets.NAME}`, `${custom.key}`
- Integrated into orchestrator for command and environment variable interpolation

### Run Detail Page Redesign ✅
- Replaced confusing swimlane layout with GitHub Actions-inspired design
- **Left panel**: Run summary (status, duration, trigger, git info) + Jobs list
- **Right panel**: Pipeline Flow DAG (compact horizontal) + Logs viewer
- Jobs list shows status icon, name, duration - click to view logs
- DAG shows pipeline flow with status colors and arrows

### Navigation Cleanup ✅
- Removed redundant "Runs" nav item from sidebar
- Runs are now accessed through Pipelines (as they should be)

### Stable Local Access ✅
- Changed API service from ClusterIP to NodePort
- Fixed NodePort at 30080
- Access BuildIt at **http://localhost:30080** (no port forwarding needed)

---

## Current State

### What Works
- **Pipelines**: Create, list, view, trigger runs
- **Pipeline Runs**: Execute with Docker or K8s, track stage results, view logs
- **UI**: Full dashboard, pipeline pages, run detail with DAG, settings pages
- **CLI**: `buildit validate` and `buildit run` for local execution
- **Database**: Full schema with migrations, stage results persisted

### What's Placeholder/Mock
- **Pipeline Creation Wizard**: UI exists but doesn't actually create pipelines
- **Logs in Run Detail**: Shows placeholder, not actual stage logs
- **Settings Pages**: UI complete but actions don't persist
- **Environments/Services/Targets**: UI exists, limited functionality

---

## Database Schema (Current)

```sql
-- Multi-tenancy
tenants (id, name, slug)
organizations (id, name, slug, plan, billing_email)
users (id, email, name, password_hash, email_verified)
org_memberships (org_id, user_id, role, invited_by)
api_keys (org_id, name, key_hash, key_prefix, scopes, expires_at)

-- Pipelines
pipelines (id, tenant_id, name, repository, config, created_at, updated_at)
pipeline_stages (id, pipeline_id, name, image, commands, depends_on, env, timeout_seconds)
pipeline_runs (id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at)
stage_results (id, pipeline_run_id, stage_name, status, job_id, started_at, finished_at, error_message)

-- Job Queue
job_queue (id, job_type, payload, status, attempts, scheduled_for, locked_until, locked_by)

-- Deployments
deployment_targets (id, tenant_id, name, target_type, config)
environments (id, tenant_id, name, slug, target_id)
services (id, tenant_id, environment_id, name, config)
deployments (id, service_id, pipeline_run_id, status, config, started_at, finished_at)
```

---

## Crate Structure

```
buildit/
├── crates/
│   ├── buildit-api/        # Axum server + Askama templates
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── state.rs    # AppState with repos + orchestrator
│   │   │   ├── routes/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── ui.rs       # HTML page handlers
│   │   │   │   ├── pipelines.rs # API endpoints
│   │   │   │   └── ...
│   │   │   └── ws.rs       # WebSocket handler
│   │   └── templates/
│   │       ├── base.html
│   │       └── pages/
│   │           ├── pipelines/
│   │           │   ├── list.html
│   │           │   ├── detail.html
│   │           │   ├── run.html      # Run detail with DAG + logs
│   │           │   └── new.html      # Creation wizard
│   │           └── ...
│   │
│   ├── buildit-core/       # Domain types
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── executor.rs    # JobSpec, JobHandle, Executor trait
│   │       ├── deployer.rs    # DeploymentSpec, Deployer trait
│   │       └── pipeline.rs    # Pipeline, Stage, StageAction
│   │
│   ├── buildit-config/     # KDL parsing + variable interpolation
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── kdl.rs         # KDL parser
│   │       └── variables.rs   # VariableContext, interpolation
│   │
│   ├── buildit-db/         # Database layer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── repo/
│   │       │   ├── tenant.rs
│   │       │   ├── pipeline.rs  # PipelineRepo with stage results
│   │       │   └── ...
│   │       └── migrations/
│   │
│   ├── buildit-executor/   # Job execution
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── docker.rs      # LocalDockerExecutor
│   │       └── kubernetes.rs  # KubernetesExecutor
│   │
│   ├── buildit-scheduler/  # Pipeline orchestration
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── orchestrator.rs  # PipelineOrchestrator, DAG execution
│   │       ├── queue.rs
│   │       └── worker.rs
│   │
│   └── buildit-cli/        # CLI tool
│       └── src/
│           └── main.rs
│
├── k8s/base/               # Kubernetes manifests
│   ├── namespace.yaml
│   ├── postgres.yaml
│   ├── api.yaml            # NodePort 30080
│   └── migrations-job.yaml
│
├── examples/               # Example pipeline configs
│   ├── echo.kdl
│   └── simple.kdl
│
├── Tiltfile
├── Dockerfile.dev
└── Dockerfile.migrations
```

---

## Key Files Reference

### Pipeline Execution Flow
1. `crates/buildit-api/src/routes/pipelines.rs:trigger_run()` - Creates run, spawns orchestrator
2. `crates/buildit-scheduler/src/orchestrator.rs` - Executes stages in DAG order
3. `crates/buildit-executor/src/docker.rs` or `kubernetes.rs` - Runs containers
4. `crates/buildit-db/src/repo/pipeline.rs` - Persists stage results

### UI Templates
- `templates/base.html` - Sidebar layout, theme toggle
- `templates/pages/pipelines/run.html` - Run detail with DAG + logs
- `templates/pages/pipelines/detail.html` - Pipeline with runs list

### Variable Interpolation
- `crates/buildit-config/src/variables.rs` - VariableContext, VariableContextBuilder

---

## Quick Reference

```bash
# Access UI (stable URL, no port-forward needed)
open http://localhost:30080

# Trigger a pipeline run via API
curl -X POST http://localhost:30080/api/v1/pipelines/{id}/runs \
  -H "Content-Type: application/json" \
  -d '{"branch": "main"}'

# Run pipeline locally with CLI
cargo run -p buildit-cli -- run examples/echo.kdl

# Check stage results in database
kubectl -n buildit exec -it deploy/postgres -- psql -U buildit -d buildit \
  -c "SELECT stage_name, status, started_at, finished_at FROM stage_results WHERE pipeline_run_id = 'xxx';"

# Rebuild and deploy API
docker build -t buildit-api:dev -f Dockerfile.dev .
kubectl -n buildit rollout restart deployment/api
```

---

## Next Up: Infrastructure-as-Code

### Database Tables Needed
```sql
-- Stacks (Terraform workspaces)
CREATE TABLE stacks (
    id UUID PRIMARY KEY,
    org_id UUID REFERENCES organizations(id),
    name VARCHAR(255) NOT NULL,
    repository VARCHAR(512) NOT NULL,
    path VARCHAR(512) DEFAULT '.',
    terraform_version VARCHAR(32) DEFAULT '1.6',
    auto_apply BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Stack Variables
CREATE TABLE stack_variables (
    id UUID PRIMARY KEY,
    stack_id UUID REFERENCES stacks(id) ON DELETE CASCADE,
    key VARCHAR(255) NOT NULL,
    value TEXT,
    sensitive BOOLEAN DEFAULT false,
    UNIQUE(stack_id, key)
);

-- Stack Runs (plan/apply)
CREATE TABLE stack_runs (
    id UUID PRIMARY KEY,
    stack_id UUID REFERENCES stacks(id),
    run_type VARCHAR(32) NOT NULL, -- 'plan', 'apply', 'destroy'
    status VARCHAR(32) NOT NULL,   -- 'pending', 'planning', 'planned', 'applying', 'succeeded', 'failed'
    plan_output TEXT,
    apply_output TEXT,
    changes_add INT DEFAULT 0,
    changes_change INT DEFAULT 0,
    changes_destroy INT DEFAULT 0,
    triggered_by UUID REFERENCES users(id),
    approved_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ
);

-- State Storage
CREATE TABLE stack_state (
    id UUID PRIMARY KEY,
    stack_id UUID REFERENCES stacks(id) UNIQUE,
    state_json JSONB NOT NULL,
    serial BIGINT DEFAULT 0,
    lock_id VARCHAR(255),
    locked_by VARCHAR(255),
    locked_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Link stacks to environments
ALTER TABLE environments ADD COLUMN stack_id UUID REFERENCES stacks(id);
```

### Implementation Order
1. Add database migrations for stack tables
2. Create Stack repository with CRUD
3. Add Stack list/detail UI pages
4. Create TerraformExecutor (wraps terraform CLI)
5. Implement plan workflow with diff viewer
6. Implement apply workflow with approval
7. Wire Stack → Environment provisioning
