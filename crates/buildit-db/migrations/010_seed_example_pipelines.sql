-- Seed 10 example pipelines demonstrating different DAG patterns
-- Uses the default tenant created in 006_seed_data.sql

-- Get default tenant ID
DO $$
DECLARE
    v_tenant_id UUID;
    v_pipeline_id UUID;
    v_run_id UUID;
BEGIN
    SELECT id INTO v_tenant_id FROM tenants WHERE slug = 'default';

    -- ==========================================================================
    -- Pipeline 1: Simple Linear (checkout -> build -> test -> deploy)
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'simple-linear', 'github.com/example/simple-app', '{"description": "Simple linear pipeline"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'build', 'node:20', ARRAY['npm install', 'npm run build'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'test', 'node:20', ARRAY['npm test'], ARRAY['build']),
    (gen_random_uuid(), v_pipeline_id, 'deploy', 'alpine/k8s', ARRAY['kubectl apply -f k8s/'], ARRAY['test']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'succeeded', '{"branch": "main", "commit_sha": "abc1234", "commit_message": "Initial commit"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '5 minutes', NOW() - INTERVAL '4 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'build', 'succeeded', NOW() - INTERVAL '4 minutes 58 seconds', NOW() - INTERVAL '3 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'test', 'succeeded', NOW() - INTERVAL '3 minutes 28 seconds', NOW() - INTERVAL '2 minutes 43 seconds'),
    (gen_random_uuid(), v_run_id, 'deploy', 'succeeded', NOW() - INTERVAL '2 minutes 43 seconds', NOW() - INTERVAL '2 minutes 23 seconds');

    -- ==========================================================================
    -- Pipeline 2: Fan-out (checkout -> 4 parallel jobs -> report)
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'parallel-tests', 'github.com/example/tested-app', '{"description": "Fan-out to parallel test jobs"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'lint', 'node:20', ARRAY['npm run lint'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'unit-test', 'node:20', ARRAY['npm run test:unit'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'integration-test', 'node:20', ARRAY['npm run test:integration'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'security-scan', 'aquasec/trivy', ARRAY['trivy fs .'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'report', 'node:20', ARRAY['npm run report'], ARRAY['lint', 'unit-test', 'integration-test', 'security-scan']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "main", "commit_sha": "def5678", "commit_message": "Add comprehensive tests"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '3 minutes', NOW() - INTERVAL '2 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'lint', 'succeeded', NOW() - INTERVAL '2 minutes 58 seconds', NOW() - INTERVAL '2 minutes 43 seconds'),
    (gen_random_uuid(), v_run_id, 'unit-test', 'succeeded', NOW() - INTERVAL '2 minutes 58 seconds', NOW() - INTERVAL '1 minute 38 seconds'),
    (gen_random_uuid(), v_run_id, 'integration-test', 'succeeded', NOW() - INTERVAL '2 minutes 58 seconds', NOW() - INTERVAL '48 seconds'),
    (gen_random_uuid(), v_run_id, 'security-scan', 'succeeded', NOW() - INTERVAL '2 minutes 58 seconds', NOW() - INTERVAL '2 minutes 13 seconds'),
    (gen_random_uuid(), v_run_id, 'report', 'running', NOW() - INTERVAL '10 seconds', NULL);

    -- ==========================================================================
    -- Pipeline 3: Diamond Pattern (multi-platform build)
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'multi-platform', 'github.com/example/cross-platform', '{"description": "Build for multiple platforms"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'build-linux', 'rust:1.75', ARRAY['cargo build --target x86_64-unknown-linux-gnu'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'build-macos', 'rust:1.75', ARRAY['cargo build --target x86_64-apple-darwin'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'build-windows', 'rust:1.75', ARRAY['cargo build --target x86_64-pc-windows-msvc'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'package', 'alpine', ARRAY['tar -czf release.tar.gz target/*/release/'], ARRAY['build-linux', 'build-macos', 'build-windows']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "release/v2.0", "commit_sha": "ghi9012", "commit_message": "Release v2.0.0"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '6 minutes', NOW() - INTERVAL '5 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'build-linux', 'succeeded', NOW() - INTERVAL '5 minutes 58 seconds', NOW() - INTERVAL '3 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'build-macos', 'succeeded', NOW() - INTERVAL '5 minutes 58 seconds', NOW() - INTERVAL '3 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'build-windows', 'succeeded', NOW() - INTERVAL '5 minutes 58 seconds', NOW() - INTERVAL '2 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'package', 'running', NOW() - INTERVAL '30 seconds', NULL);

    -- ==========================================================================
    -- Pipeline 4: Multi-stage with Mixed Dependencies
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'full-ci', 'github.com/example/enterprise-app', '{"description": "Full CI with lint, typecheck, and tests"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'install-deps', 'node:20', ARRAY['npm ci'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'lint', 'node:20', ARRAY['npm run lint'], ARRAY['install-deps']),
    (gen_random_uuid(), v_pipeline_id, 'typecheck', 'node:20', ARRAY['npm run typecheck'], ARRAY['install-deps']),
    (gen_random_uuid(), v_pipeline_id, 'unit-test', 'node:20', ARRAY['npm run test:unit'], ARRAY['install-deps']),
    (gen_random_uuid(), v_pipeline_id, 'build', 'node:20', ARRAY['npm run build'], ARRAY['lint', 'typecheck']),
    (gen_random_uuid(), v_pipeline_id, 'e2e-test', 'cypress/included', ARRAY['npm run test:e2e'], ARRAY['build', 'unit-test']),
    (gen_random_uuid(), v_pipeline_id, 'deploy', 'alpine/k8s', ARRAY['kubectl apply -f k8s/'], ARRAY['e2e-test']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "feature/new-ui", "commit_sha": "jkl3456", "commit_message": "Redesign dashboard"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '8 minutes', NOW() - INTERVAL '7 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'install-deps', 'succeeded', NOW() - INTERVAL '7 minutes 58 seconds', NOW() - INTERVAL '7 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'lint', 'succeeded', NOW() - INTERVAL '7 minutes 28 seconds', NOW() - INTERVAL '7 minutes 8 seconds'),
    (gen_random_uuid(), v_run_id, 'typecheck', 'succeeded', NOW() - INTERVAL '7 minutes 28 seconds', NOW() - INTERVAL '7 minutes 3 seconds'),
    (gen_random_uuid(), v_run_id, 'unit-test', 'succeeded', NOW() - INTERVAL '7 minutes 28 seconds', NOW() - INTERVAL '6 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'build', 'succeeded', NOW() - INTERVAL '7 minutes 3 seconds', NOW() - INTERVAL '5 minutes 33 seconds'),
    (gen_random_uuid(), v_run_id, 'e2e-test', 'running', NOW() - INTERVAL '3 minutes', NULL),
    (gen_random_uuid(), v_run_id, 'deploy', 'pending', NULL, NULL);

    -- ==========================================================================
    -- Pipeline 5: Microservices Build
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'microservices', 'github.com/example/microservices', '{"description": "Build and test multiple services"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'api-build', 'golang:1.21', ARRAY['cd api && go build'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'api-test', 'golang:1.21', ARRAY['cd api && go test ./...'], ARRAY['api-build']),
    (gen_random_uuid(), v_pipeline_id, 'web-build', 'node:20', ARRAY['cd web && npm run build'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'web-test', 'node:20', ARRAY['cd web && npm test'], ARRAY['web-build']),
    (gen_random_uuid(), v_pipeline_id, 'worker-build', 'python:3.11', ARRAY['cd worker && pip install -r requirements.txt'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'worker-test', 'python:3.11', ARRAY['cd worker && pytest'], ARRAY['worker-build']),
    (gen_random_uuid(), v_pipeline_id, 'integration', 'docker/compose', ARRAY['docker-compose -f test/docker-compose.yml up --abort-on-container-exit'], ARRAY['api-test', 'web-test', 'worker-test']),
    (gen_random_uuid(), v_pipeline_id, 'deploy-staging', 'alpine/k8s', ARRAY['kubectl apply -f k8s/staging/'], ARRAY['integration']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "main", "commit_sha": "mno7890", "commit_message": "Update API endpoints"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '10 minutes', NOW() - INTERVAL '9 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'api-build', 'succeeded', NOW() - INTERVAL '9 minutes 58 seconds', NOW() - INTERVAL '8 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'api-test', 'succeeded', NOW() - INTERVAL '8 minutes 58 seconds', NOW() - INTERVAL '8 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'web-build', 'succeeded', NOW() - INTERVAL '9 minutes 58 seconds', NOW() - INTERVAL '9 minutes 13 seconds'),
    (gen_random_uuid(), v_run_id, 'web-test', 'succeeded', NOW() - INTERVAL '9 minutes 13 seconds', NOW() - INTERVAL '8 minutes 53 seconds'),
    (gen_random_uuid(), v_run_id, 'worker-build', 'succeeded', NOW() - INTERVAL '9 minutes 58 seconds', NOW() - INTERVAL '9 minutes 8 seconds'),
    (gen_random_uuid(), v_run_id, 'worker-test', 'succeeded', NOW() - INTERVAL '9 minutes 8 seconds', NOW() - INTERVAL '8 minutes 43 seconds'),
    (gen_random_uuid(), v_run_id, 'integration', 'running', NOW() - INTERVAL '2 minutes', NULL),
    (gen_random_uuid(), v_run_id, 'deploy-staging', 'pending', NULL, NULL);

    -- ==========================================================================
    -- Pipeline 6: Matrix Build with Failure
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'node-matrix', 'github.com/example/node-lib', '{"description": "Test across Node.js versions"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'node-18', 'node:18', ARRAY['npm ci', 'npm test'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'node-20', 'node:20', ARRAY['npm ci', 'npm test'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'node-22', 'node:22', ARRAY['npm ci', 'npm test'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'publish', 'node:20', ARRAY['npm publish'], ARRAY['node-18', 'node-20', 'node-22']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'failed', '{"branch": "main", "commit_sha": "pqr1234", "commit_message": "Bump dependencies"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at, error_message) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '4 minutes', NOW() - INTERVAL '3 minutes 58 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'node-18', 'succeeded', NOW() - INTERVAL '3 minutes 58 seconds', NOW() - INTERVAL '2 minutes 48 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'node-20', 'succeeded', NOW() - INTERVAL '3 minutes 58 seconds', NOW() - INTERVAL '2 minutes 53 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'node-22', 'failed', NOW() - INTERVAL '3 minutes 58 seconds', NOW() - INTERVAL '3 minutes 13 seconds', 'Test failed: Cannot use import outside a module'),
    (gen_random_uuid(), v_run_id, 'publish', 'skipped', NULL, NULL, 'Skipped due to failed dependency');

    -- ==========================================================================
    -- Pipeline 7: Monorepo Pipeline
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'monorepo', 'github.com/example/monorepo', '{"description": "Build interdependent packages"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'core-build', 'node:20', ARRAY['cd packages/core && npm run build'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'core-test', 'node:20', ARRAY['cd packages/core && npm test'], ARRAY['core-build']),
    (gen_random_uuid(), v_pipeline_id, 'utils-build', 'node:20', ARRAY['cd packages/utils && npm run build'], ARRAY['core-build']),
    (gen_random_uuid(), v_pipeline_id, 'utils-test', 'node:20', ARRAY['cd packages/utils && npm test'], ARRAY['utils-build']),
    (gen_random_uuid(), v_pipeline_id, 'cli-build', 'node:20', ARRAY['cd packages/cli && npm run build'], ARRAY['core-build', 'utils-build']),
    (gen_random_uuid(), v_pipeline_id, 'cli-test', 'node:20', ARRAY['cd packages/cli && npm test'], ARRAY['cli-build']),
    (gen_random_uuid(), v_pipeline_id, 'web-build', 'node:20', ARRAY['cd packages/web && npm run build'], ARRAY['core-build', 'utils-build']),
    (gen_random_uuid(), v_pipeline_id, 'web-test', 'node:20', ARRAY['cd packages/web && npm test'], ARRAY['web-build']),
    (gen_random_uuid(), v_pipeline_id, 'publish-all', 'node:20', ARRAY['npx lerna publish'], ARRAY['core-test', 'utils-test', 'cli-test', 'web-test']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "main", "commit_sha": "stu5678", "commit_message": "Update shared components"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '7 minutes', NOW() - INTERVAL '6 minutes 57 seconds'),
    (gen_random_uuid(), v_run_id, 'core-build', 'succeeded', NOW() - INTERVAL '6 minutes 57 seconds', NOW() - INTERVAL '6 minutes 17 seconds'),
    (gen_random_uuid(), v_run_id, 'core-test', 'succeeded', NOW() - INTERVAL '6 minutes 17 seconds', NOW() - INTERVAL '5 minutes 47 seconds'),
    (gen_random_uuid(), v_run_id, 'utils-build', 'succeeded', NOW() - INTERVAL '6 minutes 17 seconds', NOW() - INTERVAL '5 minutes 52 seconds'),
    (gen_random_uuid(), v_run_id, 'utils-test', 'succeeded', NOW() - INTERVAL '5 minutes 52 seconds', NOW() - INTERVAL '5 minutes 32 seconds'),
    (gen_random_uuid(), v_run_id, 'cli-build', 'succeeded', NOW() - INTERVAL '5 minutes 52 seconds', NOW() - INTERVAL '5 minutes 17 seconds'),
    (gen_random_uuid(), v_run_id, 'cli-test', 'succeeded', NOW() - INTERVAL '5 minutes 17 seconds', NOW() - INTERVAL '4 minutes 32 seconds'),
    (gen_random_uuid(), v_run_id, 'web-build', 'succeeded', NOW() - INTERVAL '5 minutes 52 seconds', NOW() - INTERVAL '4 minutes 52 seconds'),
    (gen_random_uuid(), v_run_id, 'web-test', 'running', NOW() - INTERVAL '1 minute 30 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'publish-all', 'pending', NULL, NULL);

    -- ==========================================================================
    -- Pipeline 8: CI/CD with Environments
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'deploy-envs', 'github.com/example/webapp', '{"description": "Deploy through dev, staging, prod"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'build', 'node:20', ARRAY['npm ci', 'npm run build'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'unit-test', 'node:20', ARRAY['npm test'], ARRAY['build']),
    (gen_random_uuid(), v_pipeline_id, 'deploy-dev', 'alpine/k8s', ARRAY['kubectl apply -f k8s/dev/'], ARRAY['unit-test']),
    (gen_random_uuid(), v_pipeline_id, 'smoke-test-dev', 'cypress/included', ARRAY['cypress run --env=dev'], ARRAY['deploy-dev']),
    (gen_random_uuid(), v_pipeline_id, 'deploy-staging', 'alpine/k8s', ARRAY['kubectl apply -f k8s/staging/'], ARRAY['smoke-test-dev']),
    (gen_random_uuid(), v_pipeline_id, 'smoke-test-staging', 'cypress/included', ARRAY['cypress run --env=staging'], ARRAY['deploy-staging']),
    (gen_random_uuid(), v_pipeline_id, 'deploy-prod', 'alpine/k8s', ARRAY['kubectl apply -f k8s/prod/'], ARRAY['smoke-test-staging']),
    (gen_random_uuid(), v_pipeline_id, 'smoke-test-prod', 'cypress/included', ARRAY['cypress run --env=prod'], ARRAY['deploy-prod']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "main", "commit_sha": "vwx9012", "commit_message": "Production release v3.1"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '15 minutes', NOW() - INTERVAL '14 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'build', 'succeeded', NOW() - INTERVAL '14 minutes 58 seconds', NOW() - INTERVAL '13 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'unit-test', 'succeeded', NOW() - INTERVAL '13 minutes 28 seconds', NOW() - INTERVAL '12 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'deploy-dev', 'succeeded', NOW() - INTERVAL '12 minutes 28 seconds', NOW() - INTERVAL '11 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'smoke-test-dev', 'succeeded', NOW() - INTERVAL '11 minutes 58 seconds', NOW() - INTERVAL '9 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'deploy-staging', 'succeeded', NOW() - INTERVAL '9 minutes 58 seconds', NOW() - INTERVAL '9 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'smoke-test-staging', 'succeeded', NOW() - INTERVAL '9 minutes 28 seconds', NOW() - INTERVAL '7 minutes 28 seconds'),
    (gen_random_uuid(), v_run_id, 'deploy-prod', 'running', NOW() - INTERVAL '45 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'smoke-test-prod', 'pending', NULL, NULL);

    -- ==========================================================================
    -- Pipeline 9: Complex Full-Stack Pipeline
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'fullstack-app', 'github.com/example/fullstack', '{"description": "Full-stack with backend and frontend paths"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'detect-changes', 'alpine', ARRAY['./scripts/detect-changes.sh'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'backend-lint', 'golang:1.21', ARRAY['cd backend && golangci-lint run'], ARRAY['detect-changes']),
    (gen_random_uuid(), v_pipeline_id, 'backend-test', 'golang:1.21', ARRAY['cd backend && go test ./...'], ARRAY['detect-changes']),
    (gen_random_uuid(), v_pipeline_id, 'backend-build', 'golang:1.21', ARRAY['cd backend && go build -o bin/server'], ARRAY['backend-lint', 'backend-test']),
    (gen_random_uuid(), v_pipeline_id, 'frontend-lint', 'node:20', ARRAY['cd frontend && npm run lint'], ARRAY['detect-changes']),
    (gen_random_uuid(), v_pipeline_id, 'frontend-test', 'node:20', ARRAY['cd frontend && npm test'], ARRAY['detect-changes']),
    (gen_random_uuid(), v_pipeline_id, 'frontend-build', 'node:20', ARRAY['cd frontend && npm run build'], ARRAY['frontend-lint', 'frontend-test']),
    (gen_random_uuid(), v_pipeline_id, 'docker-build', 'docker', ARRAY['docker build -t app:$SHA .'], ARRAY['backend-build', 'frontend-build']),
    (gen_random_uuid(), v_pipeline_id, 'push-registry', 'docker', ARRAY['docker push registry.io/app:$SHA'], ARRAY['docker-build']),
    (gen_random_uuid(), v_pipeline_id, 'deploy-k8s', 'alpine/k8s', ARRAY['kubectl set image deployment/app app=registry.io/app:$SHA'], ARRAY['push-registry']),
    (gen_random_uuid(), v_pipeline_id, 'notify', 'alpine', ARRAY['./scripts/notify-slack.sh'], ARRAY['deploy-k8s']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "develop", "commit_sha": "yza3456", "commit_message": "Implement user dashboard"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '12 minutes', NOW() - INTERVAL '11 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'detect-changes', 'succeeded', NOW() - INTERVAL '11 minutes 58 seconds', NOW() - INTERVAL '11 minutes 53 seconds'),
    (gen_random_uuid(), v_run_id, 'backend-lint', 'succeeded', NOW() - INTERVAL '11 minutes 53 seconds', NOW() - INTERVAL '11 minutes 38 seconds'),
    (gen_random_uuid(), v_run_id, 'backend-test', 'succeeded', NOW() - INTERVAL '11 minutes 53 seconds', NOW() - INTERVAL '10 minutes 53 seconds'),
    (gen_random_uuid(), v_run_id, 'backend-build', 'succeeded', NOW() - INTERVAL '10 minutes 53 seconds', NOW() - INTERVAL '9 minutes 33 seconds'),
    (gen_random_uuid(), v_run_id, 'frontend-lint', 'succeeded', NOW() - INTERVAL '11 minutes 53 seconds', NOW() - INTERVAL '11 minutes 43 seconds'),
    (gen_random_uuid(), v_run_id, 'frontend-test', 'succeeded', NOW() - INTERVAL '11 minutes 53 seconds', NOW() - INTERVAL '11 minutes 8 seconds'),
    (gen_random_uuid(), v_run_id, 'frontend-build', 'succeeded', NOW() - INTERVAL '11 minutes 8 seconds', NOW() - INTERVAL '10 minutes 18 seconds'),
    (gen_random_uuid(), v_run_id, 'docker-build', 'succeeded', NOW() - INTERVAL '9 minutes 33 seconds', NOW() - INTERVAL '7 minutes 33 seconds'),
    (gen_random_uuid(), v_run_id, 'push-registry', 'running', NOW() - INTERVAL '30 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'deploy-k8s', 'pending', NULL, NULL),
    (gen_random_uuid(), v_run_id, 'notify', 'pending', NULL, NULL);

    -- ==========================================================================
    -- Pipeline 10: Release Pipeline with Parallel Publishing
    -- ==========================================================================
    v_pipeline_id := gen_random_uuid();
    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'release', 'github.com/example/cli-tool', '{"description": "Release to multiple registries"}');

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (gen_random_uuid(), v_pipeline_id, 'checkout', 'alpine/git', ARRAY['git clone $REPO_URL .'], ARRAY[]::TEXT[]),
    (gen_random_uuid(), v_pipeline_id, 'version-bump', 'node:20', ARRAY['npm version patch'], ARRAY['checkout']),
    (gen_random_uuid(), v_pipeline_id, 'build', 'rust:1.75', ARRAY['cargo build --release'], ARRAY['version-bump']),
    (gen_random_uuid(), v_pipeline_id, 'test', 'rust:1.75', ARRAY['cargo test'], ARRAY['build']),
    (gen_random_uuid(), v_pipeline_id, 'package-npm', 'node:20', ARRAY['npm pack'], ARRAY['test']),
    (gen_random_uuid(), v_pipeline_id, 'package-docker', 'docker', ARRAY['docker build -t cli:$VERSION .'], ARRAY['test']),
    (gen_random_uuid(), v_pipeline_id, 'package-binary', 'rust:1.75', ARRAY['cargo build --release --target x86_64-unknown-linux-musl'], ARRAY['test']),
    (gen_random_uuid(), v_pipeline_id, 'publish-npm', 'node:20', ARRAY['npm publish'], ARRAY['package-npm']),
    (gen_random_uuid(), v_pipeline_id, 'publish-docker', 'docker', ARRAY['docker push registry.io/cli:$VERSION'], ARRAY['package-docker']),
    (gen_random_uuid(), v_pipeline_id, 'publish-github', 'alpine', ARRAY['gh release create v$VERSION ./target/release/cli'], ARRAY['package-binary']),
    (gen_random_uuid(), v_pipeline_id, 'create-release', 'alpine', ARRAY['./scripts/create-release-notes.sh'], ARRAY['publish-npm', 'publish-docker', 'publish-github']),
    (gen_random_uuid(), v_pipeline_id, 'announce', 'alpine', ARRAY['./scripts/announce.sh'], ARRAY['create-release']);

    v_run_id := gen_random_uuid();
    INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info) VALUES
    (v_run_id, v_pipeline_id, 1, 'running', '{"branch": "main", "commit_sha": "bcd7890", "commit_message": "Release v1.5.0"}');

    INSERT INTO stage_results (id, pipeline_run_id, stage_name, status, started_at, finished_at) VALUES
    (gen_random_uuid(), v_run_id, 'checkout', 'succeeded', NOW() - INTERVAL '9 minutes', NOW() - INTERVAL '8 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'version-bump', 'succeeded', NOW() - INTERVAL '8 minutes 58 seconds', NOW() - INTERVAL '8 minutes 48 seconds'),
    (gen_random_uuid(), v_run_id, 'build', 'succeeded', NOW() - INTERVAL '8 minutes 48 seconds', NOW() - INTERVAL '6 minutes 48 seconds'),
    (gen_random_uuid(), v_run_id, 'test', 'succeeded', NOW() - INTERVAL '6 minutes 48 seconds', NOW() - INTERVAL '5 minutes 18 seconds'),
    (gen_random_uuid(), v_run_id, 'package-npm', 'succeeded', NOW() - INTERVAL '5 minutes 18 seconds', NOW() - INTERVAL '4 minutes 58 seconds'),
    (gen_random_uuid(), v_run_id, 'package-docker', 'succeeded', NOW() - INTERVAL '5 minutes 18 seconds', NOW() - INTERVAL '4 minutes 18 seconds'),
    (gen_random_uuid(), v_run_id, 'package-binary', 'succeeded', NOW() - INTERVAL '5 minutes 18 seconds', NOW() - INTERVAL '4 minutes 33 seconds'),
    (gen_random_uuid(), v_run_id, 'publish-npm', 'succeeded', NOW() - INTERVAL '4 minutes 58 seconds', NOW() - INTERVAL '4 minutes 43 seconds'),
    (gen_random_uuid(), v_run_id, 'publish-docker', 'succeeded', NOW() - INTERVAL '4 minutes 18 seconds', NOW() - INTERVAL '3 minutes 48 seconds'),
    (gen_random_uuid(), v_run_id, 'publish-github', 'running', NOW() - INTERVAL '20 seconds', NULL),
    (gen_random_uuid(), v_run_id, 'create-release', 'pending', NULL, NULL),
    (gen_random_uuid(), v_run_id, 'announce', 'pending', NULL, NULL);

END $$;
