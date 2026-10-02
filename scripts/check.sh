#!/usr/bin/env bash
set -euo pipefail

if ! command -v node >/dev/null 2>&1; then
  printf '%s\n' 'error: Node.js is required to validate the GNOME Shell extension.' >&2
  exit 127
fi

printf '%s\n' '==> GNOME Shell extension syntax'
node --input-type=module --check < extension/extension.js

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
