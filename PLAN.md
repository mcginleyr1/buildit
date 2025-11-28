# BuildIt - Unified DevOps Platform

A Rust-based platform combining CI/CD pipelines (like CircleCI), Infrastructure-as-Code management (like Spacelift), and GitOps deployments (like ArgoCD) into a single, cohesive system.

---

## Vision

**The Problem:** Modern DevOps requires multiple disconnected tools:
- CircleCI/GitHub Actions for CI pipelines
- Spacelift/Terraform Cloud for infrastructure
- ArgoCD/Flux for Kubernetes deployments

**BuildIt's Solution:** One platform where:
- Infrastructure provisions environments (Terraform)
- Pipelines build artifacts (CI)
- Applications deploy to environments (GitOps)
- Everything is connected and traceable

---

## Core User Journey

```
1. Connect Repository (GitHub/GitLab OAuth)
         │
         ▼
2. Auto-Detect Configuration
   ├── *.tf files → Create Stack
   ├── .buildit.kdl → Create Pipeline  
   └── k8s/*.yaml → Create Application
         │
         ▼
3. Stack runs Terraform → Creates Infrastructure
   └── Outputs: cluster_endpoint, database_url, etc.
         │
         ▼
4. Pipeline builds code → Produces Artifacts
   └── Outputs: docker image, binaries, etc.
         │
         ▼
5. Application deploys to Environment
   └── Uses Stack outputs + Pipeline artifacts
         │
         ▼
6. Continuous monitoring, drift detection, rollbacks
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              BuildIt                                     │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐               │
│   │ Repository  │     │   Stack     │     │ Application │               │
│   │  Service    │     │  Service    │     │  Service    │               │
│   │             │     │ (Terraform) │     │  (GitOps)   │               │
│   │ • Clone     │     │             │     │             │               │
│   │ • Scan      │     │ • Init      │     │ • Sync      │               │
│   │ • Webhooks  │     │ • Plan      │     │ • Diff      │               │
│   │             │     │ • Apply     │     │ • Rollback  │               │
│   └──────┬──────┘     └──────┬──────┘     └──────┬──────┘               │
│          │                   │                   │                       │
│          │    ┌──────────────┼───────────────────┘                       │
│          │    │              │                                           │
│          ▼    ▼              ▼                                           │
│   ┌─────────────────────────────────────────────────────────────┐       │
│   │                    Pipeline Service                          │       │
│   │                                                              │       │
│   │  • Stage execution (Docker/Kubernetes pods)                  │       │
│   │  • Artifact storage (images, binaries)                       │       │
│   │  • Log streaming (WebSocket)                                 │       │
│   │  • Caching                                                   │       │
│   └─────────────────────────────────────────────────────────────┘       │
│                              │                                           │
│   ┌──────────────────────────┼──────────────────────────────────┐       │
│   │                    Worker Pool                               │       │
│   │                                                              │       │
│   │   ┌────────────┐  ┌────────────┐  ┌────────────┐            │       │
│   │   │  Docker    │  │ Kubernetes │  │ Terraform  │            │       │
│   │   │  Runner    │  │  Job Pod   │  │  Runner    │            │       │
│   │   └────────────┘  └────────────┘  └────────────┘            │       │
│   └─────────────────────────────────────────────────────────────┘       │
│                                                                          │
│   ┌─────────────────────────────────────────────────────────────┐       │
│   │                    Target Registry                           │       │
│   │                                                              │       │
│   │  • Kubernetes clusters (kubeconfig, contexts)               │       │
│   │  • Cloud providers (AWS, GCP, Azure credentials)            │       │
│   │  • Container registries (Docker Hub, ECR, GCR)              │       │
│   │  • Secrets (encrypted, scoped)                              │       │
│   └─────────────────────────────────────────────────────────────┘       │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Core Entities

### Repository
Connected git repository. Source of truth for all configuration.

```rust
Repository {
    id, organization_id,
    provider,           // github, gitlab, bitbucket
    full_name,          // "owner/repo"
    clone_url,
    default_branch,
    webhook_secret,
    detected_config,    // What was found: terraform, pipeline, k8s
}
```

### Stack (Infrastructure-as-Code)
Like Spacelift - manages Terraform/OpenTofu workspaces.

```rust
Stack {
    id, tenant_id, repository_id,
    name,
    path,               // Path to .tf files in repo
    terraform_version,
    auto_apply,         // Apply automatically after plan?
    variables,          // TF_VAR_* values
    outputs,            // Extracted after apply
}

StackRun {
    id, stack_id,
    run_type,           // plan, apply, destroy
    status,             // pending, planning, needs_approval, applying, succeeded, failed
    plan_output,
    resources_to_add/change/destroy,
    triggered_by,       // user or webhook
    approved_by,
}
```

**Key Features:**
- Plan on PR, apply on merge
- Drift detection (scheduled plans)
- State locking
- Outputs flow to Environments

### Pipeline (CI)
Like CircleCI - builds, tests, produces artifacts.

```rust
Pipeline {
    id, tenant_id, repository_id,
    name,
    config,             // KDL configuration
    triggers,           // push, pr, schedule
}

PipelineRun {
    id, pipeline_id, number,
    status,
    git_sha, git_branch,
    stages: Vec<StageResult>,
    artifacts: Vec<Artifact>,
}

Stage {
    name, image,
    commands,
    depends_on,         // DAG dependencies
    needs,              // Artifacts from previous stages
}
```

**Key Features:**
- DAG-based stage execution
- Docker or Kubernetes pods
- Artifact passing between stages
- Image building and registry push

### Application (GitOps)
Like ArgoCD - syncs desired state to Kubernetes.

```rust
Application {
    id, tenant_id, repository_id,
    name,
    path,               // Path to k8s manifests
    environment_id,     // Where to deploy
    sync_policy,        // auto, manual
    health_status,
    sync_status,        // synced, out_of_sync, unknown
}

ApplicationResource {
    id, application_id,
    kind,               // Deployment, Service, ConfigMap
    name, namespace,
    status,
    live_state,         // Current state in cluster
    desired_state,      // State in git
}
```

**Key Features:**
- Watches git for changes
- Compares desired vs live state
- Automatic or manual sync
- Health monitoring
- Rollback to previous revision

### Environment
Logical deployment target (dev, staging, prod).

```rust
Environment {
    id, tenant_id,
    name, slug,
    stack_id,           // Provisioned by which stack?
    stack_outputs,      // cluster_endpoint, etc.
    variables,          // Environment-specific vars
    secrets,            // Encrypted secrets
}
```

### Target
Physical infrastructure where things run.

```rust
Target {
    id, organization_id,
    name,
    target_type,        // kubernetes, fly, cloudrun, lambda
    config,             // Connection details (encrypted)
    region,
}
```

---

## Data Flow: How Everything Connects

```
┌──────────────────────────────────────────────────────────────────────┐
│                           Repository                                  │
│                        (github.com/acme/app)                         │
└────────────────────────────────┬─────────────────────────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│     Stack       │    │    Pipeline     │    │   Application   │
│ (terraform/)    │    │ (.buildit.kdl)  │    │ (k8s/deploy/)   │
└────────┬────────┘    └────────┬────────┘    └────────┬────────┘
         │                      │                      │
         ▼                      ▼                      │
┌─────────────────┐    ┌─────────────────┐            │
│   terraform     │    │  docker build   │            │
│   apply         │    │  push to ECR    │            │
└────────┬────────┘    └────────┬────────┘            │
         │                      │                      │
         ▼                      ▼                      │
┌─────────────────┐    ┌─────────────────┐            │
│    Outputs:     │    │   Artifact:     │            │
│ cluster_url     │    │ acme/app:v1.2.3 │            │
│ database_url    │    └────────┬────────┘            │
└────────┬────────┘             │                     │
         │                      │                     │
         └──────────┬───────────┘                     │
                    │                                 │
                    ▼                                 │
         ┌─────────────────┐                          │
         │   Environment   │◄─────────────────────────┘
         │   (production)  │
         │                 │
         │ cluster: $stack.cluster_url
         │ image: $pipeline.artifact
         └────────┬────────┘
                  │
                  ▼
         ┌─────────────────┐
         │    kubectl      │
         │    apply        │
         │  (via Target)   │
         └─────────────────┘
```

---

## UI Structure

```
BuildIt
├── Dashboard
│   ├── Recent activity
│   ├── Quick stats (runs, deployments, drift)
│   └── Alerts (failed runs, out-of-sync apps)
│
├── Repositories
│   ├── Connected repos list
│   └── [repo] detail
│       ├── Overview (detected config)
│       ├── Stacks (Terraform workspaces)
│       ├── Pipelines (CI workflows)
│       └── Applications (GitOps)
│
├── Infrastructure
│   ├── Stacks
│   │   └── [stack] detail
│   │       ├── Runs (plan/apply history)
│   │       ├── State (current resources)
│   │       ├── Variables
│   │       └── Outputs
│   ├── Targets (K8s clusters, cloud accounts)
│   └── Secrets
│
├── Pipelines
│   ├── All pipelines list
│   └── [pipeline] detail
│       ├── Runs (execution history)
│       ├── Configuration
│       └── Artifacts
│
├── Applications
│   ├── All applications list
│   └── [app] detail
│       ├── Sync status
│       ├── Resources (live vs desired)
│       ├── History
│       └── Rollback
│
├── Environments
│   ├── dev / staging / production
│   └── [env] detail
│       ├── Deployed services
│       ├── Stack outputs
│       └── Variables
│
└── Settings
    ├── Organization
    ├── Team & Permissions
    ├── Integrations (GitHub, Slack)
    └── Billing
```

---

## Implementation Phases

### Phase 1: Git Foundation ✅ (Partially Complete)
**Goal:** Connect repos, detect config, receive webhooks

- [x] GitHub OAuth flow
- [x] Repository table and API
- [ ] **Git clone service** (clone repos to analyze)
- [ ] **Config scanner** (detect .tf, .buildit.kdl, k8s/)
- [ ] **Webhook processor** (trigger appropriate actions)
- [ ] Repository detail UI

### Phase 2: Stack Execution (Spacelift-like)
**Goal:** Full Terraform lifecycle management

- [ ] **Terraform runner** (containerized terraform execution)
- [ ] **Plan workflow** (run plan, show diff, wait for approval)
- [ ] **Apply workflow** (apply with locking)
- [ ] **State management** (store in DB or external backend)
- [ ] **Output extraction** (parse outputs, store for use)
- [ ] **Drift detection** (scheduled plans)
- [ ] Stack UI (runs, state viewer, variables)

### Phase 3: Pipeline Execution (CircleCI-like)
**Goal:** Robust CI pipeline execution

- [x] Basic Docker/K8s execution (exists)
- [ ] **Git clone in jobs** (clone repo into executor)
- [ ] **Artifact storage** (S3-compatible, between stages)
- [ ] **Build caching** (layer caching, dependency caching)
- [ ] **Image building** (build & push to registry)
- [ ] **Log streaming** (real-time via WebSocket)
- [ ] Pipeline UI enhancements

### Phase 4: Application Sync (ArgoCD-like)
**Goal:** GitOps deployment to Kubernetes

- [ ] **Application entity** (new table, API, UI)
- [ ] **Manifest parser** (read K8s YAML/Helm/Kustomize)
- [ ] **Live state fetcher** (get current cluster state)
- [ ] **Diff engine** (compare desired vs live)
- [ ] **Sync engine** (apply changes to cluster)
- [ ] **Health checker** (monitor deployment health)
- [ ] **Rollback** (revert to previous revision)
- [ ] Application UI (sync status, resource tree)

### Phase 5: Integration & Polish
**Goal:** Connect everything, production-ready

- [ ] **Variable/output linking** (Stack outputs → Environment → Pipeline)
- [ ] **PR integration** (plan previews, build status)
- [ ] **Notifications** (Slack, email, webhooks)
- [ ] **RBAC** (role-based access control)
- [ ] **Audit logging** (who did what when)
- [ ] **Approval workflows** (manual gates)

### Phase 6: Advanced Features
**Goal:** Enterprise capabilities

- [ ] **Policy-as-Code** (OPA for approval policies)
- [ ] **Cost estimation** (Infracost integration)
- [ ] **Multi-cluster** (deploy to multiple clusters)
- [ ] **Progressive delivery** (canary, blue-green)
- [ ] **Secrets management** (Vault integration)

---

## Configuration Examples

### .buildit.kdl (Pipeline)
```kdl
pipeline "my-app"

stage "test" {
    image "node:20"
    run "npm install"
    run "npm test"
}

stage "build" needs="test" {
    image "docker:24"
    run "docker build -t $IMAGE ."
    run "docker push $IMAGE"
    artifact "image" value="$IMAGE"
}

stage "deploy" needs="build" {
    environment "production"
    when branch="main"
}
```

### Stack (via UI or API)
```json
{
  "name": "production-infra",
  "repository": "github.com/acme/infrastructure",
  "path": "terraform/production",
  "terraform_version": "1.6.0",
  "variables": {
    "region": "us-west-2",
    "instance_type": "t3.medium"
  },
  "auto_apply": false
}
```

### Application (via UI or API)
```json
{
  "name": "my-app",
  "repository": "github.com/acme/app",
  "path": "k8s/overlays/production",
  "environment": "production",
  "sync_policy": "auto"
}
```

---

## Tech Stack

| Component | Technology | Status |
|-----------|------------|--------|
| Language | Rust | ✅ |
| Web Framework | Axum 0.8 | ✅ |
| Database | PostgreSQL + SQLx | ✅ |
| Config Format | KDL | ✅ |
| Templating | Askama | ✅ |
| Frontend | Tailwind + htmx | ✅ |
| Real-time | WebSocket | ✅ |
| Containers | Docker (bollard) | ✅ |
| Kubernetes | kube-rs | ✅ |
| Git | git2 or shell | Planned |
| Terraform | CLI in container | Planned |
| Secrets | age encryption | Planned |

---

## Success Metrics

1. **Connect repo → first deploy < 10 minutes**
2. **Stack plan visible in PR comments**
3. **Pipeline runs in parallel where possible**
4. **Application sync < 30 seconds after merge**
5. **Drift detected within scheduled interval**
6. **Single pane of glass for infra + CI + CD**

---

## Next Steps (Immediate)

1. **Finish Git clone service** - Actually clone repos
2. **Build config scanner** - Detect Terraform, pipeline, K8s configs
3. **Process webhooks** - Trigger stacks/pipelines on push
4. **Terraform runner** - Execute terraform in containers
5. **Wire UI** - Show detected config, allow setup
