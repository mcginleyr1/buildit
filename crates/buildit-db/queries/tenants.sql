--! get_tenant_by_id : Tenant()
SELECT id, name, slug, created_at, updated_at
FROM tenants
WHERE id = :id;

--! get_tenant_by_slug : Tenant()
SELECT id, name, slug, created_at, updated_at
FROM tenants
WHERE slug = :slug;

--! list_tenants : Tenant()
SELECT id, name, slug, created_at, updated_at
FROM tenants
ORDER BY name;

--! create_tenant : Tenant()
INSERT INTO tenants (id, name, slug, created_at, updated_at)
VALUES (:id, :name, :slug, NOW(), NOW())
RETURNING id, name, slug, created_at, updated_at;

--! update_tenant : Tenant()
UPDATE tenants
SET name = :name, slug = :slug, updated_at = NOW()
WHERE id = :id
RETURNING id, name, slug, created_at, updated_at;

--! delete_tenant
DELETE FROM tenants WHERE id = :id;
