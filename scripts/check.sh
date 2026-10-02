#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  printf '%s\n' 'error: Rust/Cargo is required. Install the Rust stable toolchain first.' >&2
  exit 127
fi

printf '%s\n' '==> cargo fmt'
cargo fmt --all -- --check

printf '%s\n' '==> cargo clippy'
cargo clippy --workspace --all-targets -- -D warnings

printf '%s\n' '==> cargo test'
cargo test --workspace
