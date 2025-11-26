-- Pipeline stages definition table (defines the DAG structure)
CREATE TABLE pipeline_stages (
    id UUID PRIMARY KEY,
    pipeline_id UUID NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    image VARCHAR(512),
    commands TEXT[] NOT NULL DEFAULT '{}',
    depends_on TEXT[] NOT NULL DEFAULT '{}',
    env JSONB NOT NULL DEFAULT '{}',
    timeout_seconds INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(pipeline_id, name)
);

CREATE INDEX idx_pipeline_stages_pipeline ON pipeline_stages(pipeline_id);
