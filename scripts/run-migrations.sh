#!/bin/sh
set -e

echo "Running migrations..."

# Wait for postgres to be ready
until pg_isready -h "${PGHOST:-postgres}" -U "${PGUSER:-buildit}"; do
    echo "Waiting for postgres..."
    sleep 2
done

# Create migrations tracking table if it doesn't exist
psql -h "${PGHOST:-postgres}" -U "${PGUSER:-buildit}" -d "${PGDATABASE:-buildit}" <<'EOF'
CREATE TABLE IF NOT EXISTS _sqlx_migrations (
    version BIGINT PRIMARY KEY,
    description TEXT NOT NULL,
    installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    success BOOLEAN NOT NULL,
    checksum BYTEA NOT NULL,
    execution_time BIGINT NOT NULL
);
EOF

# Run each migration in order
cd /migrations
for file in $(ls -1 *.sql | sort); do
    version=$(echo "$file" | cut -d'_' -f1)
    description=$(echo "$file" | sed 's/^[0-9]*_//' | sed 's/\.sql$//')

    # Check if migration already applied
    applied=$(psql -h "${PGHOST:-postgres}" -U "${PGUSER:-buildit}" -d "${PGDATABASE:-buildit}" -tAc \
        "SELECT 1 FROM _sqlx_migrations WHERE version = $version")

    if [ -z "$applied" ]; then
        echo "Applying migration: $file"
        start_time=$(date +%s%N)

        if psql -h "${PGHOST:-postgres}" -U "${PGUSER:-buildit}" -d "${PGDATABASE:-buildit}" -f "$file"; then
            end_time=$(date +%s%N)
            execution_time=$((($end_time - $start_time) / 1000000))
            checksum=$(sha256sum "$file" | cut -d' ' -f1)

            psql -h "${PGHOST:-postgres}" -U "${PGUSER:-buildit}" -d "${PGDATABASE:-buildit}" <<EOF
INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
VALUES ($version, '$description', true, decode('$checksum', 'hex'), $execution_time);
EOF
            echo "Migration $file applied successfully"
        else
            echo "Migration $file failed!"
            exit 1
        fi
    else
        echo "Migration $file already applied, skipping"
    fi
done

echo "All migrations complete!"
