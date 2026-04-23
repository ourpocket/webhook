# webhook
Webhook layer for ourpocket.

## Scripts

From the project root:

- `./scripts/start-webhook-server.sh` starts the Axum webhook HTTP server.
- `./scripts/start-worker.sh` starts the worker service.
- `./scripts/start-all.sh` starts both server and worker in the same shell.
- `./scripts/test.sh` runs `cargo test --workspace`.
- `./scripts/coverage.sh` runs coverage with `cargo llvm-cov` if installed.
