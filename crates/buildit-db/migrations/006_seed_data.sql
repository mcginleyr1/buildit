-- Seed data for development/demo
-- Only runs once due to migration system

-- Get the default tenant ID (created by 001_tenants.sql)
DO $$
DECLARE
    v_tenant_id UUID;
    v_target_dev UUID := gen_random_uuid();
    v_target_staging UUID := gen_random_uuid();
    v_target_prod UUID := gen_random_uuid();
    v_target_fly UUID := gen_random_uuid();
    v_env_dev UUID := gen_random_uuid();
    v_env_staging UUID := gen_random_uuid();
    v_env_prod UUID := gen_random_uuid();
    v_svc_api UUID := gen_random_uuid();
    v_svc_web UUID := gen_random_uuid();
    v_svc_worker UUID := gen_random_uuid();
BEGIN
    -- Get default tenant
    SELECT id INTO v_tenant_id FROM tenants WHERE slug = 'default';

    IF v_tenant_id IS NULL THEN
        RAISE EXCEPTION 'Default tenant not found';
    END IF;

    -- Insert targets (deployment infrastructure)
    INSERT INTO targets (id, tenant_id, name, target_type, status, region, config) VALUES
        (v_target_dev, v_tenant_id, 'dev-cluster', 'kubernetes', 'connected', 'us-east-1', '{"context": "dev-k8s", "namespace": "buildit-dev"}'),
        (v_target_staging, v_tenant_id, 'staging-cluster', 'kubernetes', 'connected', 'us-east-1', '{"context": "staging-k8s", "namespace": "buildit-staging"}'),
        (v_target_prod, v_tenant_id, 'prod-cluster', 'kubernetes', 'connected', 'us-west-2', '{"context": "prod-k8s", "namespace": "buildit-prod"}'),
        (v_target_fly, v_tenant_id, 'fly-buildit', 'fly', 'connected', 'global', '{"org": "buildit", "app_prefix": "buildit"}');

    -- Insert environments
    INSERT INTO environments (id, tenant_id, target_id, name, health_status, config) VALUES
        (v_env_dev, v_tenant_id, v_target_dev, 'development', 'healthy', '{"auto_deploy": true}'),
        (v_env_staging, v_tenant_id, v_target_staging, 'staging', 'healthy', '{"auto_deploy": true, "requires_approval": false}'),
        (v_env_prod, v_tenant_id, v_target_prod, 'production', 'healthy', '{"auto_deploy": false, "requires_approval": true}');

    -- Insert services
    INSERT INTO services (id, tenant_id, name, image, status, config) VALUES
        (v_svc_api, v_tenant_id, 'api-server', 'ghcr.io/buildit/api:v1.2.3', 'healthy', '{"port": 8080, "replicas": 3}'),
        (v_svc_web, v_tenant_id, 'web-frontend', 'ghcr.io/buildit/web:v2.0.1', 'healthy', '{"port": 3000, "replicas": 2}'),
        (v_svc_worker, v_tenant_id, 'worker', 'ghcr.io/buildit/worker:v1.1.0', 'degraded', '{"replicas": 2}');

    -- Link services to environments
    INSERT INTO service_environments (id, service_id, environment_id, current_version, status, last_deployed_at) VALUES
        -- api-server in all envs
        (gen_random_uuid(), v_svc_api, v_env_dev, 'v1.2.3', 'healthy', NOW() - INTERVAL '30 minutes'),
        (gen_random_uuid(), v_svc_api, v_env_staging, 'v1.2.3', 'healthy', NOW() - INTERVAL '2 hours'),
        (gen_random_uuid(), v_svc_api, v_env_prod, 'v1.2.3', 'healthy', NOW() - INTERVAL '2 hours'),
        -- web-frontend in all envs
        (gen_random_uuid(), v_svc_web, v_env_dev, 'v2.0.1', 'healthy', NOW() - INTERVAL '1 day'),
        (gen_random_uuid(), v_svc_web, v_env_staging, 'v2.0.1', 'healthy', NOW() - INTERVAL '1 day'),
        (gen_random_uuid(), v_svc_web, v_env_prod, 'v2.0.1', 'healthy', NOW() - INTERVAL '1 day'),
        -- worker in dev and staging only
        (gen_random_uuid(), v_svc_worker, v_env_dev, 'v1.1.0', 'degraded', NOW() - INTERVAL '3 days'),
        (gen_random_uuid(), v_svc_worker, v_env_staging, 'v1.1.0', 'degraded', NOW() - INTERVAL '3 days');

    -- Insert deployment history
    INSERT INTO deployments (id, tenant_id, service_id, environment_id, version, commit_sha, status, started_at, finished_at, created_at) VALUES
        -- Recent api-server deployments
        (gen_random_uuid(), v_tenant_id, v_svc_api, v_env_prod, 'v1.2.3', 'abc1234', 'succeeded', NOW() - INTERVAL '2 hours' - INTERVAL '45 seconds', NOW() - INTERVAL '2 hours', NOW() - INTERVAL '2 hours'),
        (gen_random_uuid(), v_tenant_id, v_svc_api, v_env_staging, 'v1.2.3', 'abc1234', 'succeeded', NOW() - INTERVAL '3 hours' - INTERVAL '42 seconds', NOW() - INTERVAL '3 hours', NOW() - INTERVAL '3 hours'),
        (gen_random_uuid(), v_tenant_id, v_svc_api, v_env_dev, 'v1.2.3', 'abc1234', 'succeeded', NOW() - INTERVAL '4 hours' - INTERVAL '38 seconds', NOW() - INTERVAL '4 hours', NOW() - INTERVAL '4 hours'),
        -- web-frontend deployment
        (gen_random_uuid(), v_tenant_id, v_svc_web, v_env_prod, 'v2.0.1', 'def5678', 'succeeded', NOW() - INTERVAL '1 day' - INTERVAL '72 seconds', NOW() - INTERVAL '1 day', NOW() - INTERVAL '1 day'),
        (gen_random_uuid(), v_tenant_id, v_svc_web, v_env_staging, 'v2.0.1', 'def5678', 'succeeded', NOW() - INTERVAL '1 day' - INTERVAL '1 hour', NOW() - INTERVAL '1 day' - INTERVAL '1 hour' + INTERVAL '65 seconds', NOW() - INTERVAL '1 day' - INTERVAL '1 hour'),
        -- worker deployment (failed in staging)
        (gen_random_uuid(), v_tenant_id, v_svc_worker, v_env_staging, 'v1.1.0', 'ghi9012', 'failed', NOW() - INTERVAL '3 days' - INTERVAL '125 seconds', NOW() - INTERVAL '3 days', NOW() - INTERVAL '3 days'),
        (gen_random_uuid(), v_tenant_id, v_svc_worker, v_env_dev, 'v1.1.0', 'ghi9012', 'succeeded', NOW() - INTERVAL '3 days' - INTERVAL '2 hours', NOW() - INTERVAL '3 days' - INTERVAL '2 hours' + INTERVAL '55 seconds', NOW() - INTERVAL '3 days' - INTERVAL '2 hours');

END $$;
