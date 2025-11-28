-- BuildIt self-build pipeline - a real working pipeline for testing
-- This pipeline builds the BuildIt CI/CD platform itself

DO $$
DECLARE
    v_tenant_id UUID;
    v_pipeline_id UUID;
BEGIN
    SELECT id INTO v_tenant_id FROM tenants WHERE slug = 'default';

    -- ==========================================================================
    -- BuildIt Self-Build Pipeline
    -- A real pipeline that builds this CI/CD platform using Rust nightly
    -- ==========================================================================
    v_pipeline_id := 'b0000000-0000-0000-0000-000000000001'::UUID;

    -- Delete if exists (for idempotency)
    DELETE FROM pipeline_stages WHERE pipeline_id = v_pipeline_id;
    DELETE FROM pipeline_runs WHERE pipeline_id = v_pipeline_id;
    DELETE FROM pipelines WHERE id = v_pipeline_id;

    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'buildit', 'https://github.com/mcginleyr1/buildit', jsonb_build_object(
        'description', 'BuildIt CI/CD Platform - Self Build',
        'stages', jsonb_build_array(
            jsonb_build_object(
                'name', 'check',
                'needs', jsonb_build_array(),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array(
                            'rustup component add rustfmt clippy',
                            'cargo fmt --check',
                            'cargo clippy -- -D warnings'
                        ),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'test',
                'needs', jsonb_build_array('check'),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array('cargo test --workspace'),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'build',
                'needs', jsonb_build_array('test'),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array('cargo build --release --workspace'),
                        'artifacts', jsonb_build_array()
                    )
                )
            )
        ),
        'env', jsonb_build_object(
            'CARGO_TERM_COLOR', 'always',
            'RUST_BACKTRACE', '1'
        ),
        'triggers', jsonb_build_array()
    ));

    -- Pipeline stages (for DAG visualization)
    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on, env) VALUES
    (
        'b0000000-0000-0000-0001-000000000001'::UUID,
        v_pipeline_id,
        'check',
        'rustlang/rust:nightly',
        ARRAY['rustup component add rustfmt clippy', 'cargo fmt --check', 'cargo clippy -- -D warnings'],
        ARRAY[]::TEXT[],
        '{"CARGO_TERM_COLOR": "always"}'::JSONB
    ),
    (
        'b0000000-0000-0000-0001-000000000002'::UUID,
        v_pipeline_id,
        'test',
        'rustlang/rust:nightly',
        ARRAY['cargo test --workspace'],
        ARRAY['check'],
        '{"CARGO_TERM_COLOR": "always", "RUST_BACKTRACE": "1"}'::JSONB
    ),
    (
        'b0000000-0000-0000-0001-000000000003'::UUID,
        v_pipeline_id,
        'build',
        'rustlang/rust:nightly',
        ARRAY['cargo build --release --workspace'],
        ARRAY['test'],
        '{"CARGO_TERM_COLOR": "always"}'::JSONB
    );

    -- ==========================================================================
    -- Simple Echo Pipeline (for quick testing)
    -- ==========================================================================
    v_pipeline_id := 'b0000000-0000-0000-0000-000000000003'::UUID;

    DELETE FROM pipeline_stages WHERE pipeline_id = v_pipeline_id;
    DELETE FROM pipeline_runs WHERE pipeline_id = v_pipeline_id;
    DELETE FROM pipelines WHERE id = v_pipeline_id;

    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'echo-test', 'https://github.com/mcginleyr1/buildit', jsonb_build_object(
        'description', 'Quick echo test pipeline for testing executor',
        'stages', jsonb_build_array(
            jsonb_build_object(
                'name', 'hello',
                'needs', jsonb_build_array(),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'alpine:latest',
                        'commands', jsonb_build_array('echo "Hello from BuildIt!"', 'sleep 2', 'echo "Stage 1 complete"'),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'world',
                'needs', jsonb_build_array('hello'),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'alpine:latest',
                        'commands', jsonb_build_array('echo "World stage running"', 'sleep 1', 'echo "Stage 2 complete"'),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'done',
                'needs', jsonb_build_array('world'),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'alpine:latest',
                        'commands', jsonb_build_array('echo "All stages complete!"'),
                        'artifacts', jsonb_build_array()
                    )
                )
            )
        ),
        'env', jsonb_build_object(),
        'triggers', jsonb_build_array()
    ));

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (
        'b0000000-0000-0000-0003-000000000001'::UUID,
        v_pipeline_id,
        'hello',
        'alpine:latest',
        ARRAY['echo "Hello from BuildIt!"', 'sleep 2', 'echo "Stage 1 complete"'],
        ARRAY[]::TEXT[]
    ),
    (
        'b0000000-0000-0000-0003-000000000002'::UUID,
        v_pipeline_id,
        'world',
        'alpine:latest',
        ARRAY['echo "World stage running"', 'sleep 1', 'echo "Stage 2 complete"'],
        ARRAY['hello']
    ),
    (
        'b0000000-0000-0000-0003-000000000003'::UUID,
        v_pipeline_id,
        'done',
        'alpine:latest',
        ARRAY['echo "All stages complete!"'],
        ARRAY['world']
    );

    -- ==========================================================================
    -- BuildIt with Parallel Linting Pipeline (more complex DAG)
    -- ==========================================================================
    v_pipeline_id := 'b0000000-0000-0000-0000-000000000002'::UUID;

    DELETE FROM pipeline_stages WHERE pipeline_id = v_pipeline_id;
    DELETE FROM pipeline_runs WHERE pipeline_id = v_pipeline_id;
    DELETE FROM pipelines WHERE id = v_pipeline_id;

    INSERT INTO pipelines (id, tenant_id, name, repository, config) VALUES
    (v_pipeline_id, v_tenant_id, 'buildit-full', 'https://github.com/mcginleyr1/buildit', jsonb_build_object(
        'description', 'BuildIt Full CI - Parallel linting and testing',
        'stages', jsonb_build_array(
            jsonb_build_object(
                'name', 'fmt',
                'needs', jsonb_build_array(),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array('rustup component add rustfmt', 'cargo fmt --check'),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'clippy',
                'needs', jsonb_build_array(),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array('rustup component add clippy', 'cargo clippy -- -D warnings'),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'test',
                'needs', jsonb_build_array('fmt', 'clippy'),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array('cargo test --workspace'),
                        'artifacts', jsonb_build_array()
                    )
                )
            ),
            jsonb_build_object(
                'name', 'build',
                'needs', jsonb_build_array('test'),
                'manual', false,
                'when', null,
                'env', jsonb_build_object(),
                'action', jsonb_build_object(
                    'Run', jsonb_build_object(
                        'image', 'rustlang/rust:nightly',
                        'commands', jsonb_build_array('cargo build --release --workspace'),
                        'artifacts', jsonb_build_array()
                    )
                )
            )
        ),
        'env', jsonb_build_object(),
        'triggers', jsonb_build_array()
    ));

    INSERT INTO pipeline_stages (id, pipeline_id, name, image, commands, depends_on) VALUES
    (
        'b0000000-0000-0000-0002-000000000001'::UUID,
        v_pipeline_id,
        'fmt',
        'rustlang/rust:nightly',
        ARRAY['rustup component add rustfmt', 'cargo fmt --check'],
        ARRAY[]::TEXT[]
    ),
    (
        'b0000000-0000-0000-0002-000000000002'::UUID,
        v_pipeline_id,
        'clippy',
        'rustlang/rust:nightly',
        ARRAY['rustup component add clippy', 'cargo clippy -- -D warnings'],
        ARRAY[]::TEXT[]
    ),
    (
        'b0000000-0000-0000-0002-000000000003'::UUID,
        v_pipeline_id,
        'test',
        'rustlang/rust:nightly',
        ARRAY['cargo test --workspace'],
        ARRAY['fmt', 'clippy']
    ),
    (
        'b0000000-0000-0000-0002-000000000004'::UUID,
        v_pipeline_id,
        'build',
        'rustlang/rust:nightly',
        ARRAY['cargo build --release --workspace'],
        ARRAY['test']
    );

END $$;
