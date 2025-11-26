-- Job queue for scheduler
CREATE TABLE job_queue (
    id UUID PRIMARY KEY,
    pipeline_run_id UUID NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    stage_name VARCHAR(255) NOT NULL,
    priority INT NOT NULL DEFAULT 0,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    claimed_by VARCHAR(255),
    claimed_at TIMESTAMPTZ,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_job_queue_status ON job_queue(status);
CREATE INDEX idx_job_queue_priority ON job_queue(priority DESC, created_at ASC);
CREATE INDEX idx_job_queue_run ON job_queue(pipeline_run_id);
