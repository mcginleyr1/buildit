# BuildIt - CI/CD Platform Plan

A modern, Rust-based CI/CD platform with container-native builds and multi-target deployments.

## Vision

Replace Jenkins/CircleCI/Argo with a self-hosted, open-source CI/CD tool that:
- Runs natively in Kubernetes
- Supports multi-tenant deployments
- Deploys to K8s, Fly.io, Cloud Run, Lambda, etc.
- Has a modern, real-time UI
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

## Phase 1: Foundation (Core Infrastructure)

### 1.1 Project Setup
- [ ] Initialize Rust workspace with Cargo
- [ ] Set up crate structure:
  ```
  buildit/
  ├── Cargo.toml (workspace)
  ├── crates/
  │   ├── buildit-api/        # Axum web server
  │   ├── buildit-core/       # Domain types, traits
  │   ├── buildit-executor/   # Job execution
  │   ├── buildit-deployer/   # Deployment logic
  │   ├── buildit-scheduler/  # Job queue
  │   ├── buildit-config/     # KDL parsing
  │   ├── buildit-db/         # Database layer
  │   └── buildit-cli/        # CLI tool
  ```
- [ ] Configure CI for the project itself (dogfooding later)
- [ ] Set up development environment (docker-compose for local dev)

### 1.2 Database Layer
- [ ] Choose and set up PostgreSQL with SQLx
- [ ] Design core schema:
  - `tenants` - Multi-tenant support
  - `users` - User accounts
  - `pipelines` - Pipeline definitions
  - `pipeline_runs` - Execution history
  - `stages` - Individual stage results
  - `artifacts` - Artifact metadata
  - `deployments` - Deployment records
  - `secrets` - Secret metadata (not values)
- [ ] Implement migrations system
- [ ] Create repository traits and implementations

### 1.3 Configuration System
- [ ] Implement KDL parser for pipeline definitions
- [ ] Implement KDL parser for system configuration
- [ ] Define configuration schema with validation
- [ ] Support variable interpolation (`{git.sha}`, `{branch}`, etc.)
- [ ] Implement configuration inheritance/imports

---

## Phase 2: Core Domain

### 2.1 Domain Types (`buildit-core`)
- [ ] Define core types:
  ```rust
  ResourceId, Image, HealthStatus, EnvVar, EnvValue
  ```
- [ ] Define executor types:
  ```rust
  JobSpec, JobHandle, JobResult, JobStatus, LogLine
  ```
- [ ] Define deployer types:
  ```rust
  DeploymentSpec, DeploymentHandle, DeploymentState,
  DeploymentStatus, DeploymentStrategy, CanaryStep
  ```
- [ ] Define pipeline types:
  ```rust
  Pipeline, Stage, StageResult, PipelineRun, Trigger
  ```

### 2.2 Executor Trait & Implementations
- [ ] Define `Executor` trait:
  ```rust
  trait Executor: Send + Sync {
      fn name(&self) -> &'static str;
      async fn can_execute(&self, spec: &JobSpec) -> bool;
      async fn spawn(&self, spec: JobSpec) -> Result<JobHandle>;
      async fn logs(&self, handle: &JobHandle) -> Result<BoxStream<LogLine>>;
      async fn status(&self, handle: &JobHandle) -> Result<JobStatus>;
      async fn wait(&self, handle: &JobHandle) -> Result<JobResult>;
      async fn cancel(&self, handle: &JobHandle) -> Result<()>;
      async fn exec_interactive(&self, handle: &JobHandle, cmd: Vec<String>) -> Result<TerminalSession>;
  }
  ```
- [ ] Implement `LocalDockerExecutor` (for dev/small teams)
- [ ] Implement `KubernetesExecutor` (production)

### 2.3 Deployer Trait & Implementations
- [ ] Define `Deployer` trait:
  ```rust
  trait Deployer: Send + Sync {
      fn name(&self) -> &'static str;
      fn supported_strategies(&self) -> Vec<DeploymentStrategy>;
      async fn validate(&self, spec: &DeploymentSpec) -> Result<Vec<ValidationWarning>>;
      async fn deploy(&self, spec: DeploymentSpec) -> Result<DeploymentHandle>;
      async fn state(&self, handle: &DeploymentHandle) -> Result<DeploymentState>;
      async fn events(&self, handle: &DeploymentHandle) -> Result<BoxStream<DeploymentEvent>>;
      async fn rollback(&self, handle: &DeploymentHandle, target: RollbackTarget) -> Result<DeploymentHandle>;
      async fn scale(&self, handle: &DeploymentHandle, replicas: u32) -> Result<()>;
      async fn pause(&self, handle: &DeploymentHandle) -> Result<()>;
      async fn resume(&self, handle: &DeploymentHandle) -> Result<()>;
      async fn destroy(&self, handle: &DeploymentHandle) -> Result<()>;
      async fn logs(&self, handle: &DeploymentHandle, opts: LogOptions) -> Result<BoxStream<LogLine>>;
      async fn exec(&self, handle: &DeploymentHandle, instance: Option<String>, cmd: Vec<String>) -> Result<TerminalSession>;
  }
  ```
- [ ] Implement `LocalDockerDeployer` (for dev)
- [ ] Implement `KubernetesDeployer` (production)
- [ ] Implement `FlyDeployer` (Fly.io)

### 2.4 Storage Abstractions
- [ ] Define `ArtifactStore` trait:
  ```rust
  trait ArtifactStore: Send + Sync {
      async fn put(&self, key: &ArtifactKey, data: Bytes) -> Result<ArtifactRef>;
      async fn get(&self, reference: &ArtifactRef) -> Result<Bytes>;
      async fn stream(&self, reference: &ArtifactRef) -> Result<BoxStream<Bytes>>;
      async fn list(&self, run_id: &ResourceId) -> Result<Vec<ArtifactManifest>>;
      async fn delete(&self, reference: &ArtifactRef) -> Result<()>;
      async fn prune(&self, policy: RetentionPolicy) -> Result<PruneStats>;
  }
  ```
- [ ] Implement `S3ArtifactStore`
- [ ] Implement `GcsArtifactStore`
- [ ] Implement `LocalArtifactStore` (for dev)

### 2.5 Secret Store Abstraction
- [ ] Define `SecretStore` trait:
  ```rust
  trait SecretStore: Send + Sync {
      async fn get(&self, path: &str) -> Result<SecretValue>;
      async fn get_key(&self, path: &str, key: &str) -> Result<String>;
      async fn list(&self, prefix: &str) -> Result<Vec<String>>;
      async fn set(&self, path: &str, value: SecretValue) -> Result<()>;
      async fn delete(&self, path: &str) -> Result<()>;
  }
  ```
- [ ] Implement `GcpSecretManager` (default)
- [ ] Implement `AwsSecretsManager`
- [ ] Implement `VaultSecretStore`
- [ ] Implement `KubernetesSecrets`

---

## Phase 3: Pipeline Engine

### 3.1 Pipeline Parser
- [ ] Parse KDL pipeline definitions
- [ ] Build DAG from stage dependencies
- [ ] Validate pipeline structure (cycles, missing deps)
- [ ] Support matrix builds
- [ ] Support conditional execution (`when` clauses)

### 3.2 Scheduler
- [ ] Implement job queue (PostgreSQL with SKIP LOCKED or Redis)
- [ ] Priority queue support
- [ ] Concurrency limits per tenant
- [ ] Fair scheduling across tenants
- [ ] Retry logic with backoff

### 3.3 Pipeline Orchestrator
- [ ] Execute pipeline DAG
- [ ] Handle stage dependencies
- [ ] Manage artifact passing between stages
- [ ] Implement caching layer
- [ ] Handle manual approval gates
- [ ] Emit events for UI updates

### 3.4 Webhook Handling
- [ ] GitHub webhook receiver
- [ ] GitLab webhook receiver
- [ ] Bitbucket webhook receiver
- [ ] Signature verification
- [ ] Event filtering (branches, paths, etc.)

---

## Phase 4: Multi-Tenancy & Security

### 4.1 Tenant Management
- [ ] Tenant CRUD operations
- [ ] Tenant isolation (namespaces, prefixes)
- [ ] Tenant-specific configuration
- [ ] Quota management and enforcement
- [ ] Tenant onboarding flow

### 4.2 Authentication
- [ ] OIDC/OAuth2 integration
- [ ] GitHub OAuth provider
- [ ] Google OAuth provider
- [ ] Generic OIDC provider
- [ ] API token management
- [ ] Session management

### 4.3 Authorization (OPA Integration)
- [ ] OPA sidecar deployment
- [ ] Policy bundle management
- [ ] Define core policies in Rego:
  - Pipeline access control
  - Deployment permissions (env-based)
  - Secret access control
  - Admin operations
- [ ] Policy decision caching
- [ ] Audit logging

### 4.4 Workload Identity
- [ ] GKE Workload Identity support
- [ ] EKS IRSA support
- [ ] Azure Workload Identity support
- [ ] Credential rotation

---

## Phase 5: API Server

### 5.1 HTTP API (Axum)
- [ ] RESTful API design
- [ ] Request validation
- [ ] Error handling and responses
- [ ] Rate limiting
- [ ] Request tracing (OpenTelemetry)

### 5.2 API Endpoints
- [ ] Pipelines CRUD
- [ ] Pipeline runs (trigger, cancel, retry)
- [ ] Deployments (deploy, rollback, scale)
- [ ] Environments management
- [ ] Secrets management
- [ ] Tenants (admin)
- [ ] Users and permissions
- [ ] Webhooks configuration

### 5.3 WebSocket Server
- [ ] Connection management
- [ ] Authentication for WebSocket
- [ ] Event subscription (runs, deployments)
- [ ] Log streaming
- [ ] Heartbeat/keepalive

---

## Phase 6: User Interface

### 6.1 UI Foundation
- [ ] Set up Askama templates
- [ ] Configure Tailwind CSS with design tokens
- [ ] Create base layout template
- [ ] Implement sidebar navigation
- [ ] Set up htmx with WebSocket extension
- [ ] Implement command palette (Cmd+K)
- [ ] Toast notification system
- [ ] Dark/light theme toggle

### 6.2 Design System
- [ ] Color palette (dark mode first):
  ```
  --bg-base:        #0a0a0b
  --bg-raised:      #111113
  --bg-elevated:    #18181b
  --accent-primary: #818cf8
  --status-success: #22c55e
  --status-error:   #ef4444
  --status-running: #3b82f6
  ```
- [ ] Typography (Inter + JetBrains Mono)
- [ ] Component library:
  - Buttons (primary, secondary, ghost)
  - Inputs (text, select, checkbox)
  - Cards and panels
  - Tables
  - Badges and status indicators
  - Modals and dialogs
  - Dropdowns
  - Tabs

### 6.3 Pipeline Pages
- [ ] Pipeline list view
  - Filter by status, branch, repo
  - Search
  - Pagination
- [ ] Pipeline detail view
  - DAG visualization
  - Stage list with status
  - Trigger button
  - Settings link
- [ ] Run detail view
  - Stage progress
  - Live log viewer
  - Artifact downloads
  - Re-run controls
- [ ] Pipeline settings
  - Edit triggers
  - Environment variables
  - Caching configuration

### 6.4 Deployment Pages
- [ ] Deployment dashboard
  - Environment matrix view
  - Service health overview
- [ ] Environment detail
  - Services in environment
  - Recent deployments
  - Resource usage
- [ ] Deployment detail
  - Rollout progress
  - Replica status
  - Traffic distribution (canary)
  - Logs
  - Rollback button
- [ ] Deploy modal
  - Select version/image
  - Choose strategy
  - Confirmation

### 6.5 Secret Management Pages
- [ ] Secret list
- [ ] Create/edit secret (values masked)
- [ ] Secret access audit log

### 6.6 Settings Pages
- [ ] Tenant settings
- [ ] User management
- [ ] Integration settings (OAuth, webhooks)
- [ ] Executor configuration
- [ ] Deployer configuration

### 6.7 Real-time Updates
- [ ] WebSocket connection management
- [ ] htmx OOB swaps for live updates
- [ ] Reconnection handling
- [ ] Optimistic UI updates

---

## Phase 7: CLI Tool

### 7.1 CLI Foundation
- [ ] Set up clap for argument parsing
- [ ] Configuration file (~/.buildit/config)
- [ ] Authentication (login, token management)
- [ ] Output formatting (table, JSON, YAML)

### 7.2 CLI Commands
- [ ] `buildit login` - Authenticate
- [ ] `buildit pipelines list` - List pipelines
- [ ] `buildit pipelines trigger <name>` - Trigger a run
- [ ] `buildit runs list` - List runs
- [ ] `buildit runs logs <id>` - Stream logs
- [ ] `buildit runs cancel <id>` - Cancel a run
- [ ] `buildit deploy <service> <env>` - Deploy
- [ ] `buildit rollback <deployment>` - Rollback
- [ ] `buildit secrets list` - List secrets
- [ ] `buildit secrets set <key>` - Set a secret
- [ ] `buildit config validate` - Validate pipeline config

---

## Phase 8: Kubernetes Deployment

### 8.1 Helm Chart
- [ ] Chart structure
- [ ] Values schema
- [ ] API server deployment
- [ ] Scheduler deployment
- [ ] PostgreSQL (optional, can use external)
- [ ] OPA sidecar configuration
- [ ] Service accounts and RBAC
- [ ] Ingress configuration
- [ ] PodDisruptionBudgets
- [ ] HorizontalPodAutoscaler

### 8.2 Operator (Optional)
- [ ] CRD definitions (Pipeline, Deployment)
- [ ] Controller implementation
- [ ] GitOps integration

### 8.3 Observability
- [ ] Prometheus metrics
- [ ] Grafana dashboards
- [ ] OpenTelemetry tracing
- [ ] Structured logging (JSON)
- [ ] Health check endpoints

---

## Phase 9: Additional Deployers

### 9.1 Cloud Run Deployer
- [ ] Service deployment
- [ ] Traffic splitting
- [ ] Revision management
- [ ] IAM configuration

### 9.2 AWS Lambda Deployer
- [ ] Function deployment
- [ ] Alias management
- [ ] Provisioned concurrency
- [ ] Layer support

### 9.3 ECS Deployer
- [ ] Service deployment
- [ ] Task definition management
- [ ] Load balancer integration

### 9.4 Nomad Deployer
- [ ] Job deployment
- [ ] Canary support
- [ ] Constraint handling

---

## Phase 10: Advanced Features

### 10.1 Caching
- [ ] Layer caching for Docker builds
- [ ] Dependency caching (cargo, npm, etc.)
- [ ] Cache invalidation strategies
- [ ] Distributed cache (S3/GCS backed)

### 10.2 Preview Environments
- [ ] Automatic PR environments
- [ ] URL generation
- [ ] Cleanup policies
- [ ] Resource limits

### 10.3 Notifications
- [ ] Slack integration
- [ ] Discord integration
- [ ] Email notifications
- [ ] Webhook notifications
- [ ] Notification preferences

### 10.4 Insights & Analytics
- [ ] Build time trends
- [ ] Success rate metrics
- [ ] Resource utilization
- [ ] Cost estimation
- [ ] Bottleneck identification

### 10.5 Local Development
- [ ] Local runner (like `act` for GitHub Actions)
- [ ] Config validation
- [ ] Debug mode with shell access

---

## Tech Stack Summary

| Component | Technology |
|-----------|------------|
| Language | Rust |
| Web Framework | Axum |
| Database | PostgreSQL + SQLx |
| Job Queue | PostgreSQL (SKIP LOCKED) |
| Config Format | KDL |
| Templating | Askama |
| CSS | Tailwind CSS |
| Interactivity | htmx + WebSocket |
| Auth | OIDC/OAuth2 |
| Policy Engine | Open Policy Agent |
| Container Runtime | containerd / Docker |
| K8s Client | kube-rs |
| Object Store | object_store crate (S3/GCS/Azure) |
| Tracing | OpenTelemetry |
| CLI | clap |

---

## KDL Configuration Examples

### Pipeline Definition
```kdl
pipeline "my-service"

on push branches=["main" "feature/*"]
on pull_request

cache "cargo" {
    path "target"
    key "cargo-{checksum Cargo.lock}"
}

stage "test" {
    image "rust:1.75"
    run "cargo test --all"
}

stage "build" needs=["test"] {
    image "rust:1.75"
    run "cargo build --release"
    artifacts ["target/release/my-service"]
}

stage "docker" needs=["build"] {
    image-build {
        dockerfile "Dockerfile"
        tags ["registry.io/my-service:{git.sha}"]
        push true
    }
}

stage "deploy-prod" needs=["docker"] {
    when "{branch} == main"
    manual true

    deploy "kubernetes" {
        namespace "production"
        replicas 3

        strategy canary {
            step traffic=10 duration="5m"
            step traffic=50 duration="10m"
            step traffic=100
        }
    }
}
```

### System Configuration
```kdl
system {
    multi-tenant true
}

artifact-store "gcs" {
    bucket "buildit-artifacts"
    auth "workload_identity" {
        service-account "buildit@project.iam.gserviceaccount.com"
    }
}

secret-store "gcp_secret_manager" {
    project "my-project"
    auth "workload_identity"
}

policy-engine "opa" {
    mode "sidecar"
    url "http://localhost:8181"
}

executors {
    executor "kubernetes" {
        type "kubernetes"
        namespace "buildit-jobs"
    }
}

deployers {
    deployer "kubernetes-prod" {
        type "kubernetes"
        context "prod-cluster"
        allowed-namespaces ["prod-*"]
    }

    deployer "fly" {
        type "fly.io"
        org "myorg"
    }
}
```

---

## Milestones

### M1: Local Dev MVP
- Project setup
- KDL parsing
- Local Docker executor
- Basic pipeline execution
- Simple UI (pipeline list, run logs)
- SQLite for local dev

### M2: Kubernetes Ready
- PostgreSQL integration
- Kubernetes executor
- Kubernetes deployer
- Helm chart
- Authentication (single tenant)

### M3: Multi-Tenant
- Tenant management
- OPA integration
- S3/GCS artifact store
- Secret store integrations
- Quota enforcement

### M4: Production Features
- Canary deployments
- Caching layer
- Notifications
- CLI tool
- Observability

### M5: Ecosystem (Ongoing)
- Additional deployers (Fly, Cloud Run, Lambda)
- Preview environments
- Analytics
- Documentation
- Community building

---

## Open Questions

1. **Naming**: Is "BuildIt" the final name? Need to check trademarks.
2. **Licensing**: MIT? Apache 2.0? AGPL for open-core model?
3. **Pricing model**: Pure open source? Open core with enterprise features?
4. **Container builds**: Build our own or integrate with Buildkit/Kaniko?
5. **GitOps**: How deep should Argo CD-style GitOps integration go?

---

## Next Steps

1. Initialize the Rust workspace
2. Define core types in `buildit-core`
3. Implement KDL parser for pipeline definitions
4. Build local Docker executor
5. Create minimal API server
6. Build basic UI for pipeline viewing

Let's start building.
