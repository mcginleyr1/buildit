# -*- mode: Python -*-

# BuildIt Tiltfile for local Kubernetes development
# Usage: tilt up

# Use OrbStack's Kubernetes
allow_k8s_contexts('orbstack')

# Configuration
config.define_bool("no-volumes")
cfg = config.parse()

# ============================================================================
# Namespace
# ============================================================================

k8s_yaml('k8s/base/namespace.yaml')

# ============================================================================
# PostgreSQL
# ============================================================================

k8s_yaml('k8s/base/postgres.yaml')

k8s_resource(
    'postgres',
    port_forwards=['5432:5432'],
    labels=['database'],
)

# ============================================================================
# Migrations
# ============================================================================

# Build migrations image
docker_build(
    'buildit-migrations',
    '.',
    dockerfile='Dockerfile.migrations',
    only=[
        'crates/buildit-db/migrations',
    ],
)

# Custom job for migrations - runs once on startup
local_resource(
    'run-migrations',
    cmd='kubectl delete job -n buildit migrations --ignore-not-found && kubectl apply -f k8s/base/migrations-job.yaml && kubectl wait --for=condition=complete job/migrations -n buildit --timeout=120s',
    resource_deps=['postgres'],
    labels=['database'],
)

# ============================================================================
# API Server
# ============================================================================

# Build API image with live reload
docker_build(
    'buildit-api',
    '.',
    dockerfile='Dockerfile.dev',
    live_update=[
        # Sync source files
        sync('./crates', '/app/crates'),
        sync('./Cargo.toml', '/app/Cargo.toml'),
        sync('./Cargo.lock', '/app/Cargo.lock'),
        # Rebuild on changes
        run('cargo build -p buildit-api', trigger=['./crates', './Cargo.toml']),
    ],
)

k8s_yaml('k8s/base/api.yaml')

k8s_resource(
    'api',
    port_forwards=['3000:3000'],
    resource_deps=['run-migrations'],
    labels=['backend'],
)

# ============================================================================
# Resource Groups
# ============================================================================

# Group related resources for better UI
config.set_enabled_resources([
    'postgres',
    'run-migrations',
    'api',
])
