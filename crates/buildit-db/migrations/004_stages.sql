-- Stage results table
CREATE TABLE stage_results (
    id UUID PRIMARY KEY,
    pipeline_run_id UUID NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    stage_name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    job_id UUID,
    deployment_id UUID,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    error_message TEXT,
    UNIQUE(pipeline_run_id, stage_name)
);

CREATE INDEX idx_stage_results_run ON stage_results(pipeline_run_id);

-- Logs table for storing job output
CREATE TABLE logs (
    id UUID PRIMARY KEY,
    pipeline_run_id UUID NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    stage_name VARCHAR(255) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    stream VARCHAR(10) NOT NULL DEFAULT 'stdout',
    content TEXT NOT NULL
);

CREATE INDEX idx_logs_run_stage ON logs(pipeline_run_id, stage_name);
CREATE INDEX idx_logs_timestamp ON logs(timestamp);
