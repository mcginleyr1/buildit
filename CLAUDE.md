# BuildIt

A Rust-based CI/CD platform with container-native builds and multi-target deployments.

## Build & Run

```bash
cargo build                    # Build all crates
cargo build -p buildit-cli     # Build CLI only
cargo test                     # Run all tests
cargo clippy                   # Run linter
cargo fmt                      # Format code

# Run the CLI
cargo run -p buildit-cli -- --help
```

## Project Structure

```
buildit/
├── crates/
│   ├── buildit-api/        # Axum web server & REST API
│   ├── buildit-cli/        # CLI tool (binary: buildit)
│   ├── buildit-config/     # KDL configuration parsing
│   ├── buildit-core/       # Domain types & traits
│   ├── buildit-db/         # PostgreSQL database layer
│   ├── buildit-deployer/   # Deployment backends (K8s, Fly.io)
│   ├── buildit-executor/   # Job execution (K8s, Docker)
│   └── buildit-scheduler/  # Job queue & worker
├── Cargo.toml              # Workspace definition
└── PLAN.md                 # Full project roadmap
```

## Core Crates

- **buildit-core**: Domain types (`ResourceId`, `Pipeline`, `Stage`), traits (`Executor`, `Deployer`, `ArtifactStore`, `SecretStore`)
- **buildit-config**: KDL parsing for pipeline definitions and system config
- **buildit-db**: PostgreSQL with SQLx, repository pattern
- **buildit-executor**: Run CI jobs in K8s pods or local Docker
- **buildit-deployer**: Deploy to K8s, Fly.io, Cloud Run, Lambda
- **buildit-scheduler**: Job queue using PostgreSQL SKIP LOCKED
- **buildit-api**: Axum HTTP API + WebSocket for real-time updates
- **buildit-cli**: Command-line interface

## Environment Notes

- Use `jj` for version control, not `git`
- Use `eza` instead of `ls` for directory listings
- Use `pls` for privileged commands
- Kubernetes available via OrbStack (`kubectl` configured)
- PostgreSQL should run in K8s for local dev

## Configuration

Pipeline configs use KDL format (see PLAN.md for examples):
- Pipeline definitions: `buildit.kdl`
- System config: KDL-based

## Key Dependencies

- **axum**: Web framework
- **sqlx**: Async PostgreSQL
- **kube-rs**: Kubernetes client
- **kdl**: Configuration format
- **tokio**: Async runtime
- **clap**: CLI parsing
