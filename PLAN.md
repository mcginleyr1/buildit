# BuildIt - CI/CD Platform Plan

A modern, Rust-based CI/CD platform with container-native builds and multi-target deployments.

## Related Documentation

- **[IMPLEMENTATION.md](./IMPLEMENTATION.md)** - Current implementation status and completed work
- **[CLAUDE.md](./CLAUDE.md)** - Development environment and quick start guide

---

## Vision

Replace Jenkins/CircleCI/Argo with a self-hosted, open-source CI/CD tool that:
- Runs natively in Kubernetes
- Supports multi-tenant deployments
- Deploys to K8s, Fly.io, Cloud Run, Lambda, etc.
- Has a modern, real-time UI with DAG visualization
- Uses KDL for configuration (not YAML)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         API Server                              │
│                     (Axum + WebSockets)                         │
└───────────────────────────┬─────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        ▼                   ▼                   ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Scheduler   │    │   Artifact   │    │    Secret    │
│   (Queue)    │    │    Store     │    │    Store     │
└──────┬───────┘    └──────────────┘    └──────────────┘
       │
       ├─────────────────────────────────┐
       ▼                                 ▼
┌──────────────┐                 ┌──────────────┐
│   Executor   │                 │   Deployer   │
│    Pool      │                 │     Pool     │
└──────┬───────┘                 └──────┬───────┘
       │                                │
   ┌───┴───┐                        ┌───┴───┐
   ▼       ▼                        ▼       ▼
┌─────┐ ┌─────┐                  ┌─────┐ ┌─────┐
│ K8s │ │Local│                  │ K8s │ │Fly  │
│Exec │ │Docker                  │Depl │ │Depl │
└─────┘ └─────┘                  └─────┘ └─────┘
```

---

## Core Concepts

### Pipelines (CI)
- **Pipeline**: A build/test workflow defined in KDL
- **Stage**: A step in a pipeline (runs in a container)
- **Run**: An execution of a pipeline
- **DAG**: Stages can depend on other stages, forming a directed acyclic graph

### Deployments (CD)
- **Environment**: A deployment target (dev, staging, production)
- **Service**: A deployed application (K8s Deployment, Fly app, Cloud Run service)
- **Target**: Infrastructure where services run (K8s cluster, Fly org, GCP project)
- **Deployment**: A release/rollout of a service version

---

## Phase 1: Foundation ✅

### 1.1 Project Setup ✅
- [x] Rust workspace with Cargo
- [x] Crate structure (api, core, executor, deployer, scheduler, config, db, cli)
- [x] Development environment (Tilt + Kubernetes + OrbStack)

### 1.2 Database Layer ✅
- [x] PostgreSQL with SQLx
- [x] Core schema (tenants, pipelines, pipeline_runs, stages, stage_results, job_queue)
- [x] Migrations system (psql-based)
- [x] Repository pattern with Clorinde for type-safe SQL

### 1.3 Configuration System (Partial)
- [x] KDL parser for pipeline definitions (basic)
- [ ] System configuration parser
- [ ] Variable interpolation (`{git.sha}`, `{branch}`)
- [ ] Configuration validation

---

## Phase 2: Core Domain ✅

### 2.1 Domain Types ✅
- [x] Core types: `ResourceId`, `Image`, `HealthStatus`, `EnvVar`
- [x] Executor types: `JobSpec`, `JobHandle`, `JobResult`, `JobStatus`, `LogLine`
- [x] Deployer types: `DeploymentSpec`, `DeploymentHandle`, `DeploymentState`
- [x] Pipeline types: `Pipeline`, `Stage`, `StageResult`, `PipelineRun`, `Trigger`

### 2.2 Executors (Partial)
- [x] `Executor` trait
- [x] `LocalDockerExecutor` (dev/small teams)
- [ ] `KubernetesExecutor` (production) - **NEXT PRIORITY**

### 2.3 Deployers
- [x] `Deployer` trait
- [ ] `KubernetesDeployer`
- [ ] `FlyDeployer`
- [ ] `CloudRunDeployer`

### 2.4 Storage
- [ ] `ArtifactStore` trait
- [ ] S3/GCS implementations
- [ ] `SecretStore` trait
- [ ] Vault/K8s Secrets implementations

---

## Phase 3: Pipeline Engine ✅

### 3.1 Pipeline Parser ✅
- [x] KDL parsing
- [x] DAG construction
- [ ] Cycle detection
- [ ] Matrix builds
- [ ] Conditional execution (`when` clauses)

### 3.2 Scheduler ✅
- [x] PostgreSQL-based job queue
- [ ] Priority queue
- [ ] Concurrency limits
- [ ] Retry logic with backoff

### 3.3 Orchestrator ✅
- [x] DAG execution
- [x] Stage dependencies
- [x] Event emission for UI
- [ ] Artifact passing
- [ ] Caching layer
- [ ] Manual approval gates

### 3.4 Webhooks
- [ ] GitHub webhook receiver
- [ ] GitLab webhook receiver
- [ ] Signature verification
- [ ] Event filtering

---

## Phase 4: Multi-Tenancy & Security ✅

### 4.1 Multi-Tenancy Model ✅
- [x] Organizations table with billing/plan support
- [x] Users table with password hashing support
- [x] Organization memberships with roles (owner, admin, member, viewer)
- [x] API keys with scoped permissions and expiration
- [x] Repository pattern for all multi-tenancy operations

### 4.2 Tenant Management ✅
- [x] Basic CRUD
- [ ] Tenant isolation enforcement
- [ ] Quota enforcement

### 4.3 Authentication
- [ ] OIDC/OAuth2
- [ ] GitHub/Google OAuth
- [x] API tokens (database layer complete)
- [ ] Session management

### 4.4 Authorization
- [ ] OPA integration
- [ ] Policy definitions
- [ ] Audit logging

---

## Phase 5: API Server ✅

### 5.1 HTTP API ✅
- [x] RESTful design with Axum
- [x] Request validation
- [x] Error handling
- [ ] Rate limiting
- [ ] OpenTelemetry tracing

### 5.2 Endpoints ✅
- [x] Pipelines CRUD
- [x] Pipeline runs
- [x] Tenants CRUD
- [x] Deployments (environments, services, targets)
- [ ] Secrets
- [ ] Users

### 5.3 WebSocket ✅
- [x] Connection management
- [x] Event subscription
- [x] Log streaming
- [ ] Authentication
- [ ] Heartbeat

---

## Phase 6: User Interface ✅

### 6.1 Foundation ✅
- [x] Askama templates (upgraded to v0.14 with askama_web for axum 0.8)
- [x] Tailwind CSS
- [x] htmx + WebSocket
- [x] Dark/light theme

### 6.2 Design System ✅
- [x] Color palette
- [x] Typography
- [x] Sidebar navigation layout
- [x] Component library (cards, buttons, forms, tables)

### 6.3 Pipeline Pages ✅
- [x] Pipeline list
- [x] Pipeline detail with runs
- [x] Run detail with log viewer
- [x] **Pipeline creation wizard** (7-step: Source, Build, Triggers, Environment, Deployment, Notifications, Review)
- [ ] DAG visualization - Key feature
- [ ] Pipeline settings/edit

### 6.4 Deployment Pages ✅
- [x] Dashboard with stats (pipeline count, run count, success rate)
- [x] Environments list with health status
- [x] Services list with deployment status
- [x] Deployment history with filtering
- [x] Targets/Infrastructure management

### 6.5 Settings Pages ✅
- [x] General settings (organization name, slug)
- [x] Team management (members from org_memberships)
- [x] Secrets management
- [x] API tokens management
- [x] Git provider connections (GitHub, GitLab, Bitbucket placeholders)
- [x] Notification settings (Slack, webhooks placeholders)

### 6.6 UX Features
- [ ] Command palette (Cmd+K)
- [ ] Keyboard shortcuts
- [ ] Toast notifications
- [ ] Empty states (basic)
- [ ] Loading states

---

## Phase 7: CLI Tool ✅

### 7.1 Foundation ✅
- [x] clap for argument parsing
- [x] `buildit validate`
- [x] `buildit run` (local Docker)
- [ ] Config file (~/.buildit/config)
- [ ] Authentication

### 7.2 Commands
- [ ] `buildit login`
- [ ] `buildit pipelines list/trigger`
- [ ] `buildit runs list/logs/cancel`
- [ ] `buildit deploy`

---

## Phase 8: Kubernetes Deployment

### 8.1 Helm Chart
- [ ] Chart structure
- [ ] API server deployment
- [ ] Scheduler deployment
- [ ] RBAC
- [ ] Ingress

### 8.2 Local Dev ✅
- [x] Tiltfile
- [x] K8s manifests
- [x] Live reload

### 8.3 Observability
- [ ] Prometheus metrics
- [ ] Grafana dashboards
- [ ] OpenTelemetry tracing
- [x] Health endpoints

---

## Milestones

### M1: Local Dev MVP ✅
- [x] Project setup
- [x] KDL parsing
- [x] Local Docker executor
- [x] Basic pipeline execution
- [x] Simple UI
- [x] PostgreSQL integration

### M2: UI & Multi-Tenancy ✅
- [x] Sidebar navigation layout
- [x] Dashboard with stats
- [x] Multi-tenancy data model (organizations, users, memberships, API keys)
- [x] Deployment pages (environments, services, targets, history)
- [x] Settings pages (general, team, secrets, tokens, git, notifications)
- [x] Pipeline creation wizard (7-step)
- [x] All routes working

### M3: Kubernetes Ready (Next)
- [ ] KubernetesExecutor
- [ ] KubernetesDeployer
- [ ] Helm chart
- [ ] Authentication (OAuth2/OIDC)

### M4: Multi-Tenant Production
- [ ] OPA integration
- [ ] Artifact storage (S3/GCS)
- [ ] Secret management
- [ ] Quota enforcement

### M5: Advanced Features
- [ ] DAG visualization
- [ ] Canary deployments
- [ ] Caching
- [ ] Notifications (actually send them)
- [ ] Preview environments
- [ ] Additional deployers (Fly, Cloud Run)

---

## Tech Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Language | Rust | ✅ |
| Web Framework | Axum 0.8 | ✅ |
| Database | PostgreSQL + SQLx | ✅ |
| Type-safe SQL | Clorinde | ✅ |
| Job Queue | PostgreSQL | ✅ |
| Config Format | KDL | ✅ |
| Templating | Askama 0.14 + askama_web | ✅ |
| CSS | Tailwind CSS | ✅ |
| Interactivity | htmx + WebSocket | ✅ |
| Auth | OIDC/OAuth2 | ❌ |
| Policy Engine | Open Policy Agent | ❌ |
| Container Runtime | Docker (bollard) | ✅ |
| K8s Client | kube-rs | Partial |
| Object Store | object_store crate | ❌ |
| Tracing | OpenTelemetry | ❌ |
| CLI | clap | ✅ |

---

## What's Left (Priority Order)

### High Priority
1. **KubernetesExecutor** - Run pipeline jobs as K8s pods (scaffolded, needs implementation)
2. **Authentication** - OAuth2/OIDC for user login, connect to users table
3. **DAG Visualization** - Visual pipeline stage graph in run detail page

### Medium Priority
4. **KubernetesDeployer** - Deploy services to K8s clusters
5. **Artifact Storage** - S3/GCS for build outputs
6. **Secret Management** - Integrate with Vault or K8s secrets
7. **Notifications** - Actually send Slack/webhook notifications
8. **Git Webhooks** - GitHub/GitLab webhook receivers for auto-triggering

### Lower Priority
9. **Helm Chart** - Production K8s deployment
10. **OPA Integration** - Fine-grained authorization
11. **Caching Layer** - Speed up builds
12. **Matrix Builds** - Multiple configurations
13. **Preview Environments** - Per-PR deployments
14. **Additional Deployers** - Fly.io, Cloud Run, Lambda
