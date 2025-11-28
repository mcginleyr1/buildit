```
 __  __           _                  _ _   _        ____ _                 _         ____          _
|  \/  | __ _  __| | ___  __      _(_) |_| |__    / ___| | __ _ _   _  __| | ___   / ___|___   __| | ___
| |\/| |/ _` |/ _` |/ _ \ \ \ /\ / / | __| '_ \  | |   | |/ _` | | | |/ _` |/ _ \ | |   / _ \ / _` |/ _ \
| |  | | (_| | (_| |  __/  \ V  V /| | |_| | | | | |___| | (_| | |_| | (_| |  __/ | |__| (_) | (_| |  __/
|_|  |_|\__,_|\__,_|\___|   \_/\_/ |_|\__|_| |_|  \____|_|\__,_|\__,_|\__,_|\___|  \____\___/ \__,_|\___|
```

---

# BuildIt

**A modern, Rust-based CI/CD platform with container-native builds, multi-target deployments, and infrastructure-as-code management.**

BuildIt is designed to replace Jenkins/CircleCI/Argo + Spacelift/Terraform Cloud with a unified, self-hosted platform that runs natively in Kubernetes.

---

## Features

- **Container-Native Builds** - Every pipeline stage runs in isolated containers (Docker or Kubernetes Jobs)
- **KDL Configuration** - Human-readable configuration format (not YAML!)
- **DAG Execution** - Stages can depend on each other, forming a directed acyclic graph
- **Real-Time UI** - Modern web interface with htmx + WebSocket for live updates
- **Multi-Tenant** - GitHub-style organization model with teams and RBAC
- **Variable Interpolation** - Dynamic variables for git info, pipeline context, secrets, and more
- **Dual Executors** - Local Docker for development, Kubernetes Jobs for production
- **Infrastructure-as-Code** - Terraform/OpenTofu integration for stack management (planned)

---

## Quick Start

### Run a Pipeline Locally with Docker

```bash
# Clone the repository
git clone https://github.com/your-org/buildit.git
cd buildit

# Run a simple pipeline
cargo run -p buildit-cli -- run examples/echo.kdl

# Validate a pipeline configuration
cargo run -p buildit-cli -- validate examples/simple.kdl
```

### Run the API Server

```bash
# Start PostgreSQL (requires Kubernetes/OrbStack)
kubectl apply -f k8s/base/namespace.yaml
kubectl apply -f k8s/base/postgres.yaml
kubectl -n buildit port-forward svc/postgres 5432:5432 &

# Set database URL
export DATABASE_URL=postgres://buildit:buildit-dev-password@127.0.0.1:5432/buildit

# Run migrations
cd crates/buildit-db && sqlx migrate run && cd ../..

# Start the API server
cargo run -p buildit-api
```

### Using Tilt for Local Development

```bash
# Start everything with live reload
tilt up

# Access the UI
open http://localhost:30080
```

---

## Pipeline Configuration (KDL)

BuildIt uses [KDL](https://kdl.dev/) for pipeline definitions - a document language that's cleaner than YAML and more readable than JSON.

### Simple Pipeline

```kdl
pipeline "my-app"

stage "test" {
    image "rust:1.75"
    run "cargo test"
}

stage "build" needs="test" {
    image "rust:1.75"
    run "cargo build --release"
    artifacts "target/release/myapp"
}

stage "deploy" needs="build" {
    image "alpine:latest"
    run "echo 'Deploying...'"
}
```

### Pipeline with Variables

```kdl
pipeline "backend"

stage "build" {
    image "rust:1.75"
    env "GIT_SHA" "${git.sha}"
    env "BRANCH" "${git.branch}"
    run "cargo build --release"
}
```

### Supported Variables

| Context | Variables |
|---------|-----------|
| Git | `${git.sha}`, `${git.short_sha}`, `${git.branch}`, `${git.message}`, `${git.author}` |
| Pipeline | `${pipeline.id}`, `${pipeline.name}` |
| Run | `${run.id}`, `${run.number}` |
| Stage | `${stage.name}`, `${stage.index}` |
| Environment | `${env.VAR_NAME}` |
| Secrets | `${secrets.SECRET_NAME}` |

---

## Architecture

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

## Project Structure

```
buildit/
├── crates/
│   ├── buildit-api/        # Axum web server, REST API, Askama templates
│   ├── buildit-cli/        # CLI tool (binary: buildit)
│   ├── buildit-config/     # KDL configuration parsing & variable interpolation
│   ├── buildit-core/       # Domain types & traits (Pipeline, Stage, Executor)
│   ├── buildit-db/         # PostgreSQL database layer with repository pattern
│   ├── buildit-db-queries/ # SQL query definitions
│   ├── buildit-deployer/   # Deployment backends (K8s, Fly.io)
│   ├── buildit-executor/   # Job execution (Docker, Kubernetes)
│   └── buildit-scheduler/  # Job queue, worker & pipeline orchestrator
├── examples/               # Example pipeline configurations
├── k8s/                    # Kubernetes manifests
│   ├── base/               # Base resources (namespace, postgres, api)
│   └── dev/                # Development overrides
├── scripts/                # Utility scripts
├── Cargo.toml              # Workspace definition
├── Dockerfile              # Production build
├── Dockerfile.dev          # Development build with hot reload
├── Tiltfile                # Tilt configuration for local K8s development
├── PLAN.md                 # Project roadmap
└── IMPLEMENTATION.md       # Implementation status
```

---

## Crate Overview

| Crate | Description |
|-------|-------------|
| `buildit-api` | Axum-based HTTP server with REST API and HTML templates |
| `buildit-cli` | Command-line interface for running and validating pipelines |
| `buildit-config` | KDL parser and variable interpolation engine |
| `buildit-core` | Core domain types: `Pipeline`, `Stage`, `Executor` trait, `Deployer` trait |
| `buildit-db` | PostgreSQL database layer with SQLx migrations and repository pattern |
| `buildit-executor` | Job execution backends: `LocalDockerExecutor`, `KubernetesExecutor` |
| `buildit-scheduler` | Pipeline orchestrator with DAG execution and event emission |
| `buildit-deployer` | Deployment backends for K8s, Fly.io, etc. |

---

## Build & Development

### Prerequisites

- **Rust 1.85+** (uses Edition 2024)
- **Docker** (for local executor)
- **Kubernetes** (OrbStack recommended for macOS)
- **PostgreSQL** (runs in K8s or locally)

### Common Commands

```bash
# Build all crates
cargo build

# Build specific crate
cargo build -p buildit-cli

# Run all tests
cargo test

# Run linter
cargo clippy --workspace

# Format code
cargo fmt

# Run the CLI
cargo run -p buildit-cli -- --help

# Run the API server
cargo run -p buildit-api
```

### Database Setup

```bash
# Port forward PostgreSQL from K8s
kubectl -n buildit port-forward svc/postgres 5432:5432 &

# Set connection string
export DATABASE_URL=postgres://buildit:buildit-dev-password@127.0.0.1:5432/buildit

# Run migrations
cd crates/buildit-db && sqlx migrate run
```

---

## Multi-Tenancy Model

BuildIt uses a GitHub-style organization model:

```
Organization (company/account)
├── Users (via org_memberships: owner/admin/member)
├── Tenants/Workspaces (projects)
│   ├── Pipelines
│   ├── Pipeline Runs
│   ├── Services
│   ├── Environments
│   ├── Deployments
│   └── Targets
└── API Keys (org-wide or tenant-scoped)
```

### Key Database Tables

| Table | Description |
|-------|-------------|
| `organizations` | Top-level accounts with plan tiers |
| `users` | User accounts with email/password or OAuth |
| `org_memberships` | User roles within an org |
| `tenants` | Workspaces within an org |
| `pipelines` | Pipeline definitions with KDL config |
| `pipeline_runs` | Execution records with status and git info |
| `stage_results` | Individual stage execution results |
| `deployment_targets` | Infrastructure targets (K8s clusters, Fly orgs) |

---

## API Endpoints

### Pipelines

```bash
# List pipelines
curl http://localhost:30080/api/v1/pipelines

# Get pipeline
curl http://localhost:30080/api/v1/pipelines/{id}

# Trigger a run
curl -X POST http://localhost:30080/api/v1/pipelines/{id}/runs \
  -H "Content-Type: application/json" \
  -d '{"branch": "main"}'

# Get run details
curl http://localhost:30080/api/v1/pipelines/{id}/runs/{run_number}
```

### Health Check

```bash
curl http://localhost:30080/health
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Web Framework | Axum 0.8 |
| Database | PostgreSQL + SQLx |
| Config Format | KDL |
| Templating | Askama 0.14 |
| CSS | Tailwind CSS |
| Interactivity | htmx + WebSocket |
| Container Runtime | Docker (bollard) |
| K8s Client | kube-rs |
| CLI | clap |
| Async Runtime | Tokio |

---

## Roadmap

### Completed

- [x] KDL pipeline configuration
- [x] Docker and Kubernetes executors
- [x] DAG-based stage execution
- [x] PostgreSQL database with migrations
- [x] Real-time UI with WebSockets
- [x] Variable interpolation system
- [x] Multi-tenant data model

### In Progress

- [ ] Infrastructure-as-Code (Terraform/OpenTofu)
- [ ] Stack management and state storage
- [ ] Plan/Apply workflow with approval

### Planned

- [ ] Authentication (OIDC/OAuth2)
- [ ] Deployment backends (K8s, Fly.io)
- [ ] OPA policy engine
- [ ] Drift detection
- [ ] Prometheus metrics
- [ ] Helm chart for production

---

## Documentation

- [PLAN.md](./PLAN.md) - Full project roadmap and vision
- [IMPLEMENTATION.md](./IMPLEMENTATION.md) - Current implementation status
- [CLAUDE.md](./CLAUDE.md) - Development environment guide

---

## License

MIT OR Apache-2.0

---

## Contributing

Contributions are welcome! Please read the documentation files above to understand the project structure and current state before submitting PRs.

---

<p align="center">
  <strong>BuildIt</strong> - Build Better, Deploy Faster
</p>
