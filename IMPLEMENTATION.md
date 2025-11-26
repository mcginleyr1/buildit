# BuildIt Implementation Status

Current implementation status and next steps for the BuildIt CI/CD platform.

---

## Completed

### Phase 1: Database Foundation ✅

- [x] Create K8s namespace `buildit`
- [x] Deploy PostgreSQL via K8s manifest
- [x] Create `buildit` database and user
- [x] Database migrations with psql-based runner
- [x] `001_tenants.sql` - tenants table
- [x] `002_pipelines.sql` - pipelines and pipeline_runs tables
- [x] `003_job_queue.sql` - job_queue table for scheduler
- [x] `004_stages.sql` - stages and stage_results tables
- [x] `005_deployment_targets.sql` - deployment infrastructure
- [x] `006_multi_tenancy.sql` - organizations, users, memberships, API keys

### Phase 2: Core Domain Types ✅

- [x] Define core types in `buildit-core`:
  - `ResourceId`, `Pipeline`, `Stage`, `StageAction`
  - `PipelineRun`, `StageResult`, `PipelineStatus`
  - `Trigger`, `TriggerInfo`, `GitInfo`
  - `JobSpec`, `JobHandle`, `JobResult`, `JobStatus`
  - `DeploymentSpec`, `DeploymentHandle`, `DeploymentState`

### Phase 3: Local Executor (Docker) ✅

- [x] Add `bollard` crate for Docker API
- [x] Implement `LocalDockerExecutor`:
  - [x] `spawn()` - create and start container
  - [x] `logs()` - stream container logs
  - [x] `status()` - check container state
  - [x] `wait()` - wait for container exit
  - [x] `cancel()` - stop and remove container
- [x] Volume & workspace handling
- [x] Environment variable injection

### Phase 4: Pipeline Orchestrator ✅

- [x] Build execution DAG from pipeline stages
- [x] Track stage states (pending, running, completed, failed)
- [x] Execute stages respecting dependencies
- [x] `PipelineOrchestrator` in buildit-scheduler
- [x] Event channel for real-time updates

### Phase 5: API Server ✅

- [x] Create `main.rs` with server startup
- [x] Database pool initialization (SQLx)
- [x] Axum 0.8 server with graceful shutdown
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

### Phase 6: User Interface ✅

- [x] Askama 0.14 templates with askama_web for Axum 0.8 compatibility
- [x] Tailwind CSS via CDN
- [x] Base layout with sidebar navigation
- [x] Dark/light theme toggle with localStorage persistence
- [x] htmx integration for dynamic updates
- [x] WebSocket extension for live updates

**Pages Implemented:**
- [x] Dashboard (`/`) - pipeline count, run stats, success rate, recent runs
- [x] Pipelines list (`/pipelines`)
- [x] Pipeline detail (`/pipelines/{id}`) - with run history
- [x] Run detail (`/pipelines/{id}/runs/{run_id}`) - with log viewer
- [x] Pipeline creation wizard (`/pipelines/new`) - 7-step wizard
- [x] Runs list (`/runs`) - all runs across pipelines
- [x] Environments (`/environments`) - deployment environments
- [x] Services (`/services`) - deployed services
- [x] History (`/history`) - deployment history
- [x] Targets (`/targets`) - infrastructure targets
- [x] Settings - General (`/settings`)
- [x] Settings - Team (`/settings/team`) - organization members
- [x] Settings - Secrets (`/settings/secrets`)
- [x] Settings - Tokens (`/settings/tokens`) - API keys
- [x] Settings - Git (`/settings/git`) - provider connections
- [x] Settings - Notifications (`/settings/notifications`)

### Phase 7: CLI Tool ✅

- [x] Basic CLI structure with clap
- [x] `buildit validate` - validate pipeline config
- [x] `buildit run` - execute pipeline locally with Docker
- [x] Real-time log streaming in terminal
- [x] KDL config file parsing

### Phase 8: Multi-Tenancy Data Model ✅

- [x] Organizations table (id, name, slug, plan, billing_email)
- [x] Users table (id, email, name, password_hash, email_verified)
- [x] Organization memberships (org_id, user_id, role, invited_by)
- [x] API keys (org_id, name, key_hash, key_prefix, scopes, expires_at)
- [x] Repository methods for all CRUD operations
- [x] UI pages connected to database

### Development Environment ✅

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

### Code Quality ✅

- [x] Zero compiler warnings across all crates
- [x] cargo fmt applied
- [x] Consistent error handling with thiserror

---

## In Progress

Nothing currently in progress.

---

## Not Started (Priority Order)

### High Priority

#### 1. Kubernetes Executor
- [ ] Implement `KubernetesExecutor.spawn()` - create K8s Job
- [ ] Configure pod spec (image, command, env, resources)
- [ ] Implement `logs()` - stream pod logs via K8s API
- [ ] Implement `status()` - watch Job status
- [ ] Implement `wait()` - wait for Job completion
- [ ] Implement `cancel()` - delete Job

#### 2. Authentication
- [ ] OIDC/OAuth2 integration
- [ ] GitHub OAuth provider
- [ ] Google OAuth provider
- [ ] Session management with cookies
- [ ] Connect login to users table
- [ ] Protect routes with auth middleware

#### 3. DAG Visualization
- [ ] Canvas/SVG-based stage graph
- [ ] Show stage dependencies visually
- [ ] Real-time status updates on nodes
- [ ] Click to view stage logs

### Medium Priority

#### 4. Kubernetes Deployer
- [ ] Implement `KubernetesDeployer.deploy()`
- [ ] Create/update K8s Deployments
- [ ] Rolling update support
- [ ] Rollback functionality

#### 5. Artifact Storage
- [ ] `ArtifactStore` trait
- [ ] S3 implementation
- [ ] GCS implementation
- [ ] Artifact upload/download in stages

#### 6. Secret Management
- [ ] `SecretStore` trait
- [ ] Kubernetes Secrets backend
- [ ] Vault backend
- [ ] Inject secrets into job containers

#### 7. Notifications
- [ ] Slack webhook integration
- [ ] Discord webhook integration
- [ ] Custom webhook support
- [ ] Email notifications

#### 8. Git Webhooks
- [ ] GitHub webhook receiver endpoint
- [ ] GitLab webhook receiver endpoint
- [ ] Signature verification
- [ ] Auto-trigger pipelines on push/PR

### Lower Priority

#### 9. Helm Chart
- [ ] Chart structure
- [ ] API server deployment
- [ ] Scheduler deployment
- [ ] RBAC configuration
- [ ] Ingress configuration
- [ ] Values for different environments

#### 10. Advanced Pipeline Features
- [ ] Variable interpolation (`{git.sha}`, `{branch}`)
- [ ] Matrix builds
- [ ] Conditional execution (`when` clauses)
- [ ] Manual approval gates
- [ ] Caching layer

#### 11. Additional Deployers
- [ ] Fly.io deployer
- [ ] Cloud Run deployer
- [ ] Lambda deployer

#### 12. Authorization
- [ ] OPA integration
- [ ] Policy definitions
- [ ] Audit logging

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
│ (Orchestrator)│   │  (PostgreSQL)│    │   (Docker)   │
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

## Database Schema

### Core Tables
- `tenants` - Legacy tenant table (being replaced by organizations)
- `pipelines` - Pipeline definitions
- `pipeline_runs` - Pipeline execution instances
- `stages` - Stage definitions within pipelines
- `stage_results` - Stage execution results
- `job_queue` - Background job queue

### Deployment Tables
- `deployment_targets` - Infrastructure targets (K8s clusters, Fly orgs)
- `environments` - Deployment environments (dev, staging, prod)
- `services` - Deployed applications
- `deployments` - Deployment history

### Multi-Tenancy Tables
- `organizations` - Organizations/teams
- `users` - User accounts
- `org_memberships` - User-organization relationships with roles
- `api_keys` - API authentication tokens

---

## Quick Start (Local Development)

```bash
# Start local K8s cluster (OrbStack recommended)
tilt up

# Access UI
open http://localhost:3000

# Run CLI locally
cargo run -p buildit-cli -- run examples/echo.kdl

# View Tilt dashboard
open http://localhost:10350

# Run with zero warnings
cargo build
```

---

## Recent Changes

### 2024-11-26
- Fixed empty server responses (askama 0.12 -> 0.14 compatibility with axum 0.8)
- Updated askama_web to 0.14 with axum-0.8 feature
- Fixed all compiler warnings across all crates
- Applied cargo fmt to entire codebase
