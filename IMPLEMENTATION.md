# BuildIt Implementation Plan

Step-by-step plan to get BuildIt to a working MVP where we can run a pipeline locally.

## Phase 1: Database Foundation

### 1.1 PostgreSQL Setup in Kubernetes
- [ ] Create K8s namespace `buildit`
- [ ] Deploy PostgreSQL via Helm or manifest
- [ ] Create `buildit` database and user
- [ ] Test connection from local machine

### 1.2 Database Migrations
- [ ] Create `migrations/` in buildit-db with SQLx migrations
- [ ] `001_tenants.sql` - tenants table
- [ ] `002_pipelines.sql` - pipelines and pipeline_runs tables
- [ ] `003_job_queue.sql` - job_queue table for scheduler
- [ ] `004_stages.sql` - stage_results table
- [ ] Run migrations and verify schema

## Phase 2: KDL Configuration Parser

### 2.1 Pipeline Config Parsing
- [ ] Define KDL schema for pipelines
- [ ] Parse `pipeline` node (name, triggers)
- [ ] Parse `stage` nodes (name, image, commands, needs)
- [ ] Parse `cache` nodes
- [ ] Validate DAG (no cycles, valid dependencies)
- [ ] Variable interpolation (`{git.sha}`, `{branch}`)

### 2.2 System Config Parsing
- [ ] Parse executor configuration
- [ ] Parse deployer configuration
- [ ] Parse artifact/secret store config

## Phase 3: Local Executor (Docker)

### 3.1 Docker Executor Implementation
- [ ] Add `bollard` crate for Docker API
- [ ] Implement `LocalDockerExecutor`
- [ ] `spawn()` - create and start container
- [ ] `logs()` - stream container logs
- [ ] `status()` - check container state
- [ ] `wait()` - wait for container exit
- [ ] `cancel()` - stop and remove container

### 3.2 Volume & Workspace Handling
- [ ] Mount workspace directory into container
- [ ] Handle artifact collection from container
- [ ] Environment variable injection

## Phase 4: Pipeline Orchestrator

### 4.1 DAG Execution Engine
- [ ] Build execution DAG from parsed pipeline
- [ ] Topological sort for execution order
- [ ] Track stage states (pending, running, completed, failed)
- [ ] Execute stages respecting dependencies
- [ ] Handle parallel stage execution

### 4.2 Pipeline Runner
- [ ] Create `PipelineRunner` in buildit-scheduler
- [ ] Load pipeline config from file or database
- [ ] Create pipeline run record
- [ ] Execute stages via executor
- [ ] Update stage/run status in database
- [ ] Collect and store logs

## Phase 5: API Server Runnable

### 5.1 Server Binary
- [ ] Create `main.rs` in buildit-api with server startup
- [ ] Database pool initialization
- [ ] Load system configuration
- [ ] Start Axum server
- [ ] Graceful shutdown handling

### 5.2 Core API Endpoints
- [ ] `POST /api/v1/pipelines/{id}/trigger` - trigger a run
- [ ] `GET /api/v1/runs/{id}` - get run details with stages
- [ ] `GET /api/v1/runs/{id}/logs` - get run logs
- [ ] WebSocket `/ws` - subscribe to run updates

## Phase 6: CLI Integration

### 6.1 HTTP Client
- [ ] Add `reqwest` to buildit-cli
- [ ] Implement API client for all endpoints
- [ ] Config file for API URL and auth token

### 6.2 Working Commands
- [ ] `buildit validate` - validate local pipeline config
- [ ] `buildit run` - trigger pipeline locally (no server)
- [ ] `buildit pipelines trigger` - trigger via API
- [ ] `buildit runs logs --follow` - stream logs via WebSocket

## Phase 7: Kubernetes Executor

### 7.1 K8s Job Creation
- [ ] Implement `KubernetesExecutor.spawn()` - create K8s Job
- [ ] Configure pod spec (image, command, env, resources)
- [ ] Handle secrets injection
- [ ] Implement `logs()` - stream pod logs via K8s API

### 7.2 Job Lifecycle
- [ ] Implement `status()` - watch Job status
- [ ] Implement `wait()` - wait for Job completion
- [ ] Implement `cancel()` - delete Job
- [ ] Handle pod failures and restarts

## Phase 8: Real-time Updates

### 8.1 Event System
- [ ] Create event bus (tokio broadcast channel)
- [ ] Publish events on run/stage state changes
- [ ] Publish log lines as events

### 8.2 WebSocket Streaming
- [ ] Subscribe clients to run events
- [ ] Stream log lines in real-time
- [ ] Handle client disconnection gracefully

---

## MVP Definition

The MVP is complete when we can:

1. Write a `buildit.kdl` pipeline config
2. Run `buildit validate buildit.kdl` to check it
3. Run `buildit run` to execute the pipeline locally with Docker
4. See real-time logs in the terminal
5. (Optional) Run via API server with `buildit pipelines trigger`

---

## Execution Order

Start with these in order:

1. **Phase 1** - Database (need storage for runs)
2. **Phase 2** - KDL Parser (need to read pipeline configs)
3. **Phase 3** - Docker Executor (need to run jobs)
4. **Phase 4** - Pipeline Orchestrator (tie it together)
5. **Phase 6.2** - CLI `buildit run` command (local execution)

Phases 5, 7, 8 can come after we have local execution working.

---

## Next Action

Start with **Phase 1.1**: Deploy PostgreSQL to your OrbStack Kubernetes cluster.
