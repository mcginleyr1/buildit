-- Seed data for organizations and users
DO $$
DECLARE
    v_org_id UUID := gen_random_uuid();
    v_user_admin UUID := gen_random_uuid();
    v_user_dev UUID := gen_random_uuid();
    v_tenant_id UUID;
BEGIN
    -- Create default organization
    INSERT INTO organizations (id, name, slug, plan, settings) VALUES
        (v_org_id, 'BuildIt Demo', 'buildit-demo', 'pro', '{"features": ["deployments", "environments"]}');

    -- Create demo users (passwords would normally be hashed - these are placeholders)
    INSERT INTO users (id, email, name, email_verified_at, settings) VALUES
        (v_user_admin, 'admin@buildit.dev', 'Admin User', NOW(), '{"theme": "dark"}'),
        (v_user_dev, 'dev@buildit.dev', 'Developer User', NOW(), '{"theme": "dark"}');

    -- Add users to organization
    INSERT INTO org_memberships (id, organization_id, user_id, role, accepted_at) VALUES
        (gen_random_uuid(), v_org_id, v_user_admin, 'owner', NOW()),
        (gen_random_uuid(), v_org_id, v_user_dev, 'member', NOW());

    -- Update default tenant to belong to organization
    SELECT id INTO v_tenant_id FROM tenants WHERE slug = 'default';

    IF v_tenant_id IS NOT NULL THEN
        UPDATE tenants SET organization_id = v_org_id WHERE id = v_tenant_id;

        -- Add users to tenant
        INSERT INTO tenant_memberships (id, tenant_id, user_id, role) VALUES
            (gen_random_uuid(), v_tenant_id, v_user_admin, 'admin'),
            (gen_random_uuid(), v_tenant_id, v_user_dev, 'member');
    END IF;

    -- Create a sample API key (in real life, the actual key would be shown once then only hash stored)
    -- key_prefix: "bld_demo1234" (first 12 chars for display)
    -- The full key would be something like "bld_demo1234_secretparthere"
    INSERT INTO api_keys (id, organization_id, user_id, name, key_prefix, key_hash, scopes) VALUES
        (gen_random_uuid(), v_org_id, v_user_admin, 'Demo API Key', 'bld_demo1234',
         'placeholder_hash_would_be_sha256_of_full_key',
         ARRAY['read', 'write', 'pipelines:trigger']);

END $$;
