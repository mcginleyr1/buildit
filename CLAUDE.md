# BuildIt

A Rust-based CI/CD platform with container-native builds and multi-target deployments.

## Quick Start

```bash
# Run a pipeline locally with Docker
cargo run -p buildit-cli -- run examples/echo.kdl

# Validate a pipeline config
cargo run -p buildit-cli -- validate examples/simple.kdl
```

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
│   └── buildit-scheduler/  # Job queue, worker & orchestrator
├── examples/               # Example pipeline configs
├── k8s/                    # Kubernetes manifests
├── Cargo.toml              # Workspace definition
├── PLAN.md                 # Full project roadmap
└── IMPLEMENTATION.md       # Implementation plan
```

## Pipeline Configuration (KDL)

```kdl
pipeline "my-app"

stage "test" {
    image "rust:1.75"
    run "cargo test"
}

stage "build" needs="test" {
    image "rust:1.75"
    run "cargo build --release"
}
```

## Local Development

PostgreSQL runs in K8s (OrbStack):
```bash
# Start port forward (if not already running)
kubectl -n buildit port-forward svc/postgres 5432:5432 &

# Connection string
DATABASE_URL=postgres://buildit:buildit-dev-password@127.0.0.1:5432/buildit

# Run migrations
cd crates/buildit-db && sqlx migrate run
```

## Environment Notes

- Use `jj` for version control, not `git`
- Use `eza` instead of `ls` for directory listings
- Use `pls` for privileged commands
- Kubernetes available via OrbStack (`kubectl` configured)
- PostgreSQL runs in K8s namespace `buildit`

## Key Dependencies

- **axum**: Web framework
- **sqlx**: Async PostgreSQL  
- **kube-rs**: Kubernetes client
- **bollard**: Docker API
- **kdl**: Configuration format
- **tokio**: Async runtime
- **clap**: CLI parsing
