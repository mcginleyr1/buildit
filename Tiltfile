# -*- mode: Python -*-

# BuildIt Tiltfile for local Kubernetes development
# Usage: tilt up

# Use OrbStack's Kubernetes
allow_k8s_contexts('orbstack')

# OrbStack shares Docker daemon with K8s, so local images work directly

# ============================================================================
# Namespace & Secrets
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

docker_build(
    'buildit-migrations',
    '.',
    dockerfile='Dockerfile.migrations',
    only=['crates/buildit-db/migrations', 'scripts', 'Dockerfile.migrations'],
)

k8s_yaml('k8s/base/migrations-job.yaml')

k8s_resource(
    'migrations',
    resource_deps=['postgres'],
    labels=['database'],
)

# ============================================================================
# API Server
# ============================================================================

docker_build(
    'buildit-api',
    '.',
    dockerfile='Dockerfile.dev',
    ignore=['target/', '.git/', 'k8s/'],
    live_update=[
        sync('./crates', '/app/crates'),
        sync('./Cargo.toml', '/app/Cargo.toml'),
        sync('./Cargo.lock', '/app/Cargo.lock'),
        run('cd /app && cargo build -p buildit-api'),
    ],
)

k8s_yaml('k8s/base/api.yaml')

k8s_resource(
    'api',
    port_forwards=['3000:3000'],
    resource_deps=['migrations'],
    labels=['backend'],
    links=[
        link('http://localhost:3000', 'UI'),
        link('http://localhost:3000/health', 'Health'),
    ],
)
