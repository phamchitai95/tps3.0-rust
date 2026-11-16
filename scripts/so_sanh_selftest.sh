#!/usr/bin/env bash
# S7 self-test: dump fixture qlth_test → restore qlth_so_sanh → run harness.
# Không cần backup thật. Dùng trên Cloud/CI.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PGHOST="${PGHOST:-127.0.0.1}"
export PGPORT="${PGPORT:-5432}"
export PGUSER="${PGUSER:-tps}"
export PGPASSWORD="${PGPASSWORD:-tps_dev}"

echo "== Ensure fixture qlth_test"
"$ROOT/scripts/init_dev_db.sh"

TMP="$(mktemp -t qlth_fixture_XXXX.dump)"
trap 'rm -f "$TMP"' EXIT

echo "== pg_dump qlth_test → $TMP"
pg_dump -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -Fc -f "$TMP" qlth_test

echo "== Restore + so sánh"
"$ROOT/scripts/so_sanh.sh" "$TMP"
