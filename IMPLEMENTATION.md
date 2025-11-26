# BuildIt Implementation Status

Current implementation status and next steps for the BuildIt CI/CD platform.

## Completed

### Phase 1: Database Foundation

- [x] Create K8s namespace `buildit`
- [x] Deploy PostgreSQL via K8s manifest
- [x] Create `buildit` database and user
- [x] Database migrations with psql-based runner
- [x] `001_tenants.sql` - tenants table
- [x] `002_pipelines.sql` - pipelines and pipeline_runs tables
- [x] `003_job_queue.sql` - job_queue table for scheduler
- [x] `004_stages.sql` - stages and stage_results tables

### Phase 2: Core Domain Types

- [x] Define core types in `buildit-core`:
  - `ResourceId`, `Pipeline`, `Stage`, `StageAction`
  - `PipelineRun`, `StageResult`, `PipelineStatus`
  - `Trigger`, `TriggerInfo`, `GitInfo`
  - `JobSpec`, `JobHandle`, `JobResult`, `JobStatus`
  - `DeploymentSpec`, `DeploymentHandle`, `DeploymentState`

### Phase 3: Local Executor (Docker)

- [x] Add `bollard` crate for Docker API
- [x] Implement `LocalDockerExecutor`:
  - [x] `spawn()` - create and start container
  - [x] `logs()` - stream container logs
  - [x] `status()` - check container state
  - [x] `wait()` - wait for container exit
  - [x] `cancel()` - stop and remove container
- [x] Volume & workspace handling
- [x] Environment variable injection

### Phase 4: Pipeline Orchestrator

- [x] Build execution DAG from pipeline stages
- [x] Track stage states (pending, running, completed, failed)
- [x] Execute stages respecting dependencies
- [x] `PipelineOrchestrator` in buildit-scheduler
- [x] Event channel for real-time updates

### Phase 5: API Server

- [x] Create `main.rs` with server startup
- [x] Database pool initialization (SQLx)
- [x] Axum server with graceful shutdown
- [x] Core API endpoints:
  - [x] `GET /api/v1/tenants` - list tenants
  - [x] `POST /api/v1/tenants` - create tenant
  - [x] `GET /api/v1/pipelines` - list pipelines
  - [x] `POST /api/v1/pipelines` - create pipeline
  - [x] `GET /api/v1/pipelines/{id}` - get pipeline
  - [x] `POST /api/v1/pipelines/{id}/runs` - trigger run
  - [x] `GET /api/v1/pipelines/{id}/runs` - list runs
- [x] WebSocket `/ws` endpoint for real-time updates
- [x] Health check endpoints (`/health`, `/health/ready`)

### Phase 6: User Interface

- [x] Askama templates setup
- [x] Tailwind CSS via CDN
- [x] Base layout with navigation
- [x] Dark/light theme toggle with localStorage persistence
- [x] Pipeline list page (`/`)
- [x] Pipeline detail page (`/pipelines/{id}`)
- [x] Run detail page with log viewer (`/pipelines/{id}/runs/{run_id}`)
- [x] New pipeline modal
- [x] htmx integration for dynamic updates
- [x] WebSocket extension for live updates

### Phase 7: CLI Tool

- [x] Basic CLI structure with clap
- [x] `buildit validate` - validate pipeline config
- [x] `buildit run` - execute pipeline locally with Docker
- [x] Real-time log streaming in terminal
- [x] KDL config file parsing

### Development Environment

- [x] Tilt + Kubernetes local dev setup
- [x] K8s manifests (`k8s/base/`)
  - [x] namespace.yaml
  - [x] postgres.yaml (with PVC, secrets)
  - [x] api.yaml (with RBAC)
  - [x] migrations-job.yaml
  - [x] kustomization.yaml
- [x] Dockerfile.dev with cargo-watch for live reload
- [x] Dockerfile.migrations with psql
- [x] Tiltfile with resource dependencies
- [x] OrbStack Kubernetes compatibility

### Database Layer

- [x] SQLx integration with PostgreSQL
- [x] Clorinde for type-safe SQL query generation
- [x] Repository pattern (`PgTenantRepo`, `PgPipelineRepo`)
- [x] deadpool-postgres connection pooling

---

## In Progress

### Kubernetes Executor

- [ ] Implement `KubernetesExecutor.spawn()` - create K8s Job
- [ ] Configure pod spec (image, command, env, resources)
- [ ] Implement `logs()` - stream pod logs via K8s API
- [ ] Implement `status()` - watch Job status
- [ ] Implement `wait()` - wait for Job completion
- [ ] Implement `cancel()` - delete Job

---

## Not Started

### KDL Configuration Parser (Full)

- [ ] Parse `cache` nodes
- [ ] Validate DAG (no cycles, valid dependencies)
- [ ] Variable interpolation (`{git.sha}`, `{branch}`)
- [ ] System config parsing (executors, deployers)

### Deployers

- [ ] `KubernetesDeployer` implementation
- [ ] Canary deployment support
- [ ] Rollback functionality

### Multi-Tenancy & Auth

- [ ] OIDC/OAuth2 integration
- [ ] OPA policy engine
- [ ] Tenant isolation

### Advanced Features

- [ ] Artifact storage (S3/GCS)
- [ ] Secret management integration
- [ ] Caching layer
- [ ] Notifications (Slack, Discord)
- [ ] Preview environments

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         API Server                              │
│                     (Axum + WebSockets)                         │
│                      localhost:3000                             │
└───────────────────────────┬─────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Scheduler   │    │   Database   │    │   Executor   │
│ (Orchestrator)    │  (PostgreSQL) │    │   (Docker)   │
└──────────────┘    └──────────────┘    └──────────────┘
```

## Crate Structure

```
buildit/
├── crates/
│   ├── buildit-api/        # Axum web server + UI templates
│   ├── buildit-core/       # Domain types, traits
│   ├── buildit-executor/   # Job execution (Docker, K8s)
│   ├── buildit-deployer/   # Deployment logic
│   ├── buildit-scheduler/  # Pipeline orchestration
│   ├── buildit-config/     # KDL parsing
│   ├── buildit-db/         # Database layer (SQLx)
│   ├── buildit-db-queries/ # Generated Clorinde queries
│   └── buildit-cli/        # CLI tool
├── k8s/
│   └── base/               # Kubernetes manifests
├── scripts/
│   └── run-migrations.sh   # Migration runner
├── Tiltfile                # Local K8s dev config
├── Dockerfile.dev          # Dev image with cargo-watch
└── Dockerfile.migrations   # Migration job image
```

---

## Quick Start (Local Development)

```bash
# Start local K8s cluster (OrbStack recommended)
tilt up

# Access UI
open http://localhost:3000

# Run CLI locally
cargo run -p buildit-cli -- run

# View Tilt dashboard
open http://localhost:10350
```

---

## Next Steps

1. **Implement KubernetesExecutor** - Enable running jobs as K8s pods
2. **Add artifact storage** - S3/GCS for build artifacts
3. **Implement caching** - Speed up builds with dependency caching
4. **Add authentication** - OAuth2/OIDC for user login
5. **Production Dockerfile** - Multi-stage optimized build
