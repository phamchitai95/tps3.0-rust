#!/usr/bin/env bash
# Create/reset qlth_test from schema/qlth_cautruc.sql (dev Cloud / local).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOST="${PGHOST:-127.0.0.1}"
PORT="${PGPORT:-5432}"
USER="${PGUSER:-tps}"
PASS="${PGPASSWORD:-tps_dev}"
DB="${QLTH_DATABASE:-qlth_test}"
export PGPASSWORD="$PASS"

echo "== Drop/create $DB"
psql -h "$HOST" -p "$PORT" -U "$USER" -d postgres -v ON_ERROR_STOP=1 <<SQL
SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB' AND pid <> pg_backend_pid();
DROP DATABASE IF EXISTS $DB;
CREATE DATABASE $DB OWNER $USER;
SQL

echo "== Apply schema"
psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -v ON_ERROR_STOP=1 -f "$ROOT/schema/qlth_cautruc.sql"

echo "== Ensure app_meta PK + migrate_version=27"
psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -v ON_ERROR_STOP=1 <<'SQL'
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conrelid = 'app_meta'::regclass AND contype = 'p') THEN
    ALTER TABLE app_meta ADD PRIMARY KEY (key);
  END IF;
END $$;
INSERT INTO app_meta (key, value) VALUES ('migrate_version', '27')
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value;
SQL

echo "== Seed fixture"
psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -v ON_ERROR_STOP=1 -f "$ROOT/scripts/seed_dev.sql"

echo "== OK: $DB ready (migrate_version=27, admin/admin)"
