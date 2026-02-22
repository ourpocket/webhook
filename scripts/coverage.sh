#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "cargo-llvm-cov is not installed. Install with: cargo install cargo-llvm-cov"
  exit 1
fi
cargo llvm-cov --workspace --all-features --html

