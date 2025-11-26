--! enqueue_job : Job()
INSERT INTO job_queue (id, pipeline_run_id, stage_name, priority, status, created_at)
VALUES (:id, :pipeline_run_id, :stage_name, :priority, 'pending', NOW())
RETURNING id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at;

--! claim_job : Job()
UPDATE job_queue
SET status = 'running', claimed_at = NOW(), claimed_by = :claimed_by
WHERE id = (
    SELECT id FROM job_queue
    WHERE status = 'pending'
    ORDER BY priority DESC, created_at ASC
    LIMIT 1
    FOR UPDATE SKIP LOCKED
)
RETURNING id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at;

--! complete_job
UPDATE job_queue
SET status = 'completed'
WHERE id = :id;

--! fail_job
UPDATE job_queue
SET status = 'failed', error = :error
WHERE id = :id;

--! retry_job
UPDATE job_queue
SET status = 'pending', claimed_by = NULL, claimed_at = NULL, error = :error
WHERE id = :id;

--! get_job_by_id : Job()
SELECT id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at
FROM job_queue
WHERE id = :id;

--! list_jobs_by_run : Job()
SELECT id, pipeline_run_id, stage_name, priority, status, claimed_by, claimed_at, error, created_at
FROM job_queue
WHERE pipeline_run_id = :pipeline_run_id
ORDER BY created_at;

--! count_pending_jobs : PendingCount()
SELECT COUNT(*) as count
FROM job_queue
WHERE status = 'pending';
