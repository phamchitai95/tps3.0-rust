#!/usr/bin/env bash
# Restore a real Postgres dump into qlth_so_sanh for comparison (S7).
# Usage: ./scripts/restore_so_sanh.sh private_data/qlth.dump
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DUMP="${1:-}"
if [[ -z "$DUMP" || ! -f "$DUMP" ]]; then
  echo "Usage: $0 path/to/backup.dump" >&2
  echo "Put dumps in private_data/ (gitignored)." >&2
  exit 1
fi
HOST="${PGHOST:-127.0.0.1}"
PORT="${PGPORT:-5432}"
USER="${PGUSER:-tps}"
PASS="${PGPASSWORD:-tps_dev}"
DB="qlth_so_sanh"
export PGPASSWORD="$PASS"

echo "== Recreate $DB"
psql -h "$HOST" -p "$PORT" -U "$USER" -d postgres -v ON_ERROR_STOP=1 <<SQL
SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$DB' AND pid <> pg_backend_pid();
DROP DATABASE IF EXISTS $DB;
CREATE DATABASE $DB OWNER $USER;
SQL

echo "== Restore $DUMP"
if [[ "$DUMP" == *.sql ]]; then
  psql -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" -v ON_ERROR_STOP=1 -f "$DUMP"
else
  pg_restore -h "$HOST" -p "$PORT" -U "$USER" -d "$DB" --no-owner --role="$USER" "$DUMP" || true
fi

echo "== OK: restored into $DB — run cargo test / so_sanh next"
