#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run -p webhook-server &
SERVER_PID=$!
cargo run -p worker &
WORKER_PID=$!
wait "$SERVER_PID" "$WORKER_PID"

