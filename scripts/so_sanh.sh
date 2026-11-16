#!/usr/bin/env bash
# S7 — restore (optional) + smoke so sánh trên qlth_so_sanh.
# Usage:
#   ./scripts/so_sanh.sh                         # chỉ chạy check (DB đã restore)
#   ./scripts/so_sanh.sh private_data/qlth.dump  # restore rồi check
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DUMP="${1:-}"

if [[ -n "$DUMP" ]]; then
  "$ROOT/scripts/restore_so_sanh.sh" "$DUMP"
fi

export QLTH_DATABASE="${QLTH_DATABASE:-qlth_so_sanh}"
export PGHOST="${PGHOST:-127.0.0.1}"
export PGPORT="${PGPORT:-5432}"
export PGUSER="${PGUSER:-tps}"
export PGPASSWORD="${PGPASSWORD:-tps_dev}"

echo "== tps-so-sanh on $QLTH_DATABASE"
cd "$ROOT"
cargo run -q -p tps-so-sanh
