--! get_pipeline_by_id : Pipeline()
SELECT id, tenant_id, name, repository, config, created_at, updated_at
FROM pipelines
WHERE id = :id;

--! list_pipelines_by_tenant : Pipeline()
SELECT id, tenant_id, name, repository, config, created_at, updated_at
FROM pipelines
WHERE tenant_id = :tenant_id
ORDER BY name;

--! create_pipeline : Pipeline()
INSERT INTO pipelines (id, tenant_id, name, repository, config, created_at, updated_at)
VALUES (:id, :tenant_id, :name, :repository, :config, NOW(), NOW())
RETURNING id, tenant_id, name, repository, config, created_at, updated_at;

--! update_pipeline_config : Pipeline()
UPDATE pipelines
SET config = :config, updated_at = NOW()
WHERE id = :id
RETURNING id, tenant_id, name, repository, config, created_at, updated_at;

--! delete_pipeline
DELETE FROM pipelines WHERE id = :id;

--! get_run_by_id : PipelineRun()
SELECT id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at
FROM pipeline_runs
WHERE id = :id;

--! list_runs_by_pipeline : PipelineRun()
SELECT id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at
FROM pipeline_runs
WHERE pipeline_id = :pipeline_id
ORDER BY number DESC
LIMIT :limit;

--! create_run : PipelineRun()
INSERT INTO pipeline_runs (id, pipeline_id, number, status, trigger_info, git_info, created_at)
VALUES (
    :id,
    :pipeline_id,
    (SELECT COALESCE(MAX(number), 0) + 1 FROM pipeline_runs WHERE pipeline_id = :pipeline_id),
    'queued',
    :trigger_info,
    :git_info,
    NOW()
)
RETURNING id, pipeline_id, number, status, trigger_info, git_info, created_at, started_at, finished_at;

--! update_run_status
UPDATE pipeline_runs
SET status = :status
WHERE id = :id;

--! mark_run_started
UPDATE pipeline_runs
SET status = 'running', started_at = NOW()
WHERE id = :id AND started_at IS NULL;

--! mark_run_finished
UPDATE pipeline_runs
SET status = :status, finished_at = NOW()
WHERE id = :id;

--! get_next_run_number : NextRunNumber()
SELECT COALESCE(MAX(number), 0) + 1 as next_number
FROM pipeline_runs
WHERE pipeline_id = :pipeline_id;
