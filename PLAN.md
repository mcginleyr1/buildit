# BuildIt - CI/CD & Infrastructure Platform

A modern, Rust-based CI/CD platform with container-native builds, multi-target deployments, and infrastructure-as-code management.

## Related Documentation

- **[IMPLEMENTATION.md](./IMPLEMENTATION.md)** - Current implementation status and completed work
- **[CLAUDE.md](./CLAUDE.md)** - Development environment and quick start guide

---

## Vision

Replace Jenkins/CircleCI/Argo + Spacelift/Terraform Cloud with a unified, self-hosted platform that:
- Runs natively in Kubernetes
- Supports multi-tenant deployments
- Deploys to K8s, Fly.io, Cloud Run, Lambda, etc.
- Manages infrastructure via Terraform/OpenTofu
- Has a modern, real-time UI with DAG visualization
- Uses KDL for configuration (not YAML)

---

## Core Concepts

### The Big Picture

```
Stacks (IaC)              →  provision infrastructure (Terraform)
    ↓ creates
Environments              ←  deployment targets (dev/staging/prod)
    ↑ deploys to
Pipelines (CI/CD)         →  build/test/deploy code
    ↓ produces
Services                  →  running workloads
```

### Pipelines (CI/CD)
- **Pipeline**: A build/test/deploy workflow defined in KDL
- **Stage**: A step in a pipeline (runs in a container)
- **Run**: An execution of a pipeline
- **DAG**: Stages can depend on other stages, forming a directed acyclic graph

### Stacks (Infrastructure-as-Code)
- **Stack**: A Terraform workspace (git repo + path + state + variables)
- **Stack Run**: A plan/apply operation on a stack
- **Drift Detection**: Scheduled plans to detect infrastructure drift
- **Policy**: OPA-based checks before apply (cost, security, compliance)

### Environments & Deployments
- **Environment**: A deployment target (dev, staging, production) - can be provisioned by a Stack
- **Service**: A deployed application (K8s Deployment, Fly app, etc.)
- **Target**: Infrastructure where services run (K8s cluster, Fly org, GCP project)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         API Server                              │
│                     (Axum + WebSockets)                         │
│                    http://localhost:30080                       │
└───────────────────────────┬─────────────────────────────────────┘
                            │
    ┌───────────────────────┼───────────────────────────────────┐
    ▼                       ▼                       ▼           ▼
┌──────────┐         ┌──────────┐         ┌──────────┐   ┌──────────┐
│Scheduler │         │ Database │         │ Executor │   │  Stack   │
│(Pipeline │         │(Postgres)│         │ (Docker/ │   │ Runner   │
│Orchestr.)│         │          │         │   K8s)   │   │(Terraform│
└──────────┘         └──────────┘         └──────────┘   └──────────┘
                                                │               │
                                            ┌───┴───┐       ┌───┴───┐
                                            ▼       ▼       ▼       ▼
                                         ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐
                                         │Local│ │ K8s │ │ AWS │ │ GCP │
                                         │Dock.│ │ Job │ │     │ │     │
                                         └─────┘ └─────┘ └─────┘ └─────┘
```

---

## Database Schema

### Core Tables
```sql
-- Organizations & Users
organizations (id, name, slug, plan, billing_email)
users (id, email, name, password_hash)
org_memberships (org_id, user_id, role)
api_keys (org_id, name, key_hash, scopes, expires_at)

-- Pipelines (CI/CD)
pipelines (id, tenant_id, name, repository, config)
pipeline_stages (id, pipeline_id, name, image, commands, depends_on)
pipeline_runs (id, pipeline_id, number, status, trigger_info, git_info)
stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at)

-- Stacks (IaC) - NEW
stacks (id, org_id, name, repository, path, terraform_version, auto_apply)
stack_variables (id, stack_id, key, value, sensitive)
stack_runs (id, stack_id, type, status, plan_output, triggered_by)
stack_state (id, stack_id, state_json, lock_id, locked_by)

-- Environments & Deployments
environments (id, org_id, name, slug, stack_id, cluster_url, credentials)
services (id, environment_id, name, image, replicas, status)
deployments (id, service_id, pipeline_run_id, status, started_at)
deployment_targets (id, org_id, name, type, config)
```

---

## Phase 1: Foundation ✅

### 1.1 Project Setup ✅
- [x] Rust workspace with Cargo
- [x] Crate structure (api, core, executor, deployer, scheduler, config, db, cli)
- [x] Development environment (Tilt + Kubernetes + OrbStack)
- [x] NodePort service for stable local access (http://localhost:30080)

### 1.2 Database Layer ✅
- [x] PostgreSQL with SQLx
- [x] Core schema (tenants, pipelines, pipeline_runs, stages, stage_results)
- [x] Migrations system
- [x] Repository pattern

### 1.3 Configuration System ✅
- [x] KDL parser for pipeline definitions
- [x] Variable interpolation (`${git.sha}`, `${git.branch}`, `${env.VAR}`)
- [x] VariableContext with git, pipeline, run, stage, env, secrets contexts

---

## Phase 2: Pipeline Engine ✅

### 2.1 Executors ✅
- [x] `Executor` trait
- [x] `LocalDockerExecutor` (dev/small teams)
- [x] `KubernetesExecutor` (production) - runs jobs as K8s Jobs

### 2.2 Orchestrator ✅
- [x] DAG execution with topological sort
- [x] Stage dependencies
- [x] Event emission (StageStarted, StageCompleted, StageLog, PipelineCompleted)
- [x] Stage result persistence to database
- [x] Real duration tracking

### 2.3 Scheduler ✅
- [x] PostgreSQL-based job queue
- [x] Pipeline run triggering via API

---

## Phase 3: User Interface ✅

### 3.1 Foundation ✅
- [x] Askama templates with Tailwind CSS
- [x] htmx + WebSocket for real-time updates
- [x] Dark/light theme
- [x] Sidebar navigation

### 3.2 Pipeline Pages ✅
- [x] Pipeline list
- [x] Pipeline detail with recent runs
- [x] Run detail with GitHub Actions-style layout:
  - Left panel: Run summary + Jobs list
  - Right panel: Pipeline flow DAG + Logs viewer
- [x] Pipeline creation wizard (7-step)
- [x] Real stage statuses and durations from database

### 3.3 Other Pages ✅
- [x] Dashboard with stats
- [x] Environments, Services, History, Targets
- [x] Settings (General, Team, Secrets, Tokens, Git, Notifications)

---

## Phase 4: Infrastructure-as-Code (NEW - Next Priority)

### 4.1 Stack Management
- [ ] `stacks` table (repo, path, terraform version, variables)
- [ ] `stack_runs` table (plan/apply, status, output)
- [ ] `stack_state` table (terraform state storage)
- [ ] Stack CRUD API endpoints
- [ ] Stack UI pages under Infrastructure → Stacks

### 4.2 Terraform Runner
- [ ] `TerraformExecutor` - wraps terraform CLI
- [ ] Run `terraform init` on stack creation
- [ ] Run `terraform plan` and capture output
- [ ] Plan diff viewer in UI (resources to add/change/destroy)
- [ ] Manual approval workflow
- [ ] Run `terraform apply` after approval
- [ ] State locking

### 4.3 Stack → Environment Link
- [ ] Stack can provision an Environment
- [ ] Extract outputs (cluster URL, credentials) into Environment
- [ ] Environment shows "Managed by Stack X"

### 4.4 Advanced Stack Features
- [ ] Drift detection (scheduled plans)
- [ ] Cost estimation integration
- [ ] OPA policy checks before apply
- [ ] Stack dependencies (one stack uses outputs from another)
- [ ] PR-based plan previews

---

## Phase 5: Deployment Engine

### 5.1 Deployers
- [ ] `Deployer` trait
- [ ] `KubernetesDeployer` - deploy to K8s clusters
- [ ] `FlyDeployer` - deploy to Fly.io
- [ ] Rollback support

### 5.2 Pipeline → Environment Integration
- [ ] Stage can target an Environment
- [ ] Use Environment credentials for deployment
- [ ] Deployment history linked to pipeline runs

---

## Phase 6: Authentication & Security

### 6.1 Authentication
- [ ] OIDC/OAuth2 (GitHub, Google)
- [ ] Session management
- [ ] API token authentication (database layer complete)

### 6.2 Authorization
- [ ] OPA integration for fine-grained policies
- [ ] Stack/Pipeline permission policies
- [ ] Audit logging

---

## Phase 7: Production Readiness

### 7.1 Observability
- [ ] Prometheus metrics
- [ ] OpenTelemetry tracing
- [ ] Structured logging

### 7.2 Helm Chart
- [ ] Production K8s deployment
- [ ] HA configuration
- [ ] Ingress with TLS

---

## Tech Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Language | Rust | ✅ |
| Web Framework | Axum 0.8 | ✅ |
| Database | PostgreSQL + SQLx | ✅ |
| Config Format | KDL | ✅ |
| Templating | Askama 0.14 | ✅ |
| CSS | Tailwind CSS | ✅ |
| Interactivity | htmx + WebSocket | ✅ |
| Container Runtime | Docker (bollard) | ✅ |
| K8s Client | kube-rs | ✅ |
| IaC | Terraform/OpenTofu | Planned |
| Auth | OIDC/OAuth2 | Planned |
| Policy Engine | Open Policy Agent | Planned |

---

## Milestones

### M1: Local Dev MVP ✅
- [x] KDL parsing, Docker executor, basic UI, PostgreSQL

### M2: Production Pipeline Engine ✅
- [x] K8s executor, stage result persistence, real-time UI, variable interpolation

### M3: Infrastructure-as-Code (Current)
- [ ] Stack management, Terraform runner, state storage, plan/apply workflow

### M4: Full Platform
- [ ] Authentication, deployers, environment integration

### M5: Enterprise Features
- [ ] OPA policies, drift detection, cost estimation, audit logs
