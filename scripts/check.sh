#!/usr/bin/env bash
set -euo pipefail

if ! command -v node >/dev/null 2>&1; then
  printf '%s\n' 'error: Node.js is required to validate the GNOME Shell extension.' >&2
  exit 127
fi

printf '%s\n' '==> GNOME Shell extension syntax'
node --input-type=module --check < extension/extension.js

printf '%s\n' '==> M7 acceptance harness syntax'
for script in scripts/m7-acceptance.sh scripts/m7-report.sh scripts/m7-finalize.sh \
  scripts/build-m7-gnome46-validation-deb.sh; do
  bash -n "$script"
done
python3 -c 'import ast, pathlib; ast.parse(pathlib.Path("scripts/m7_acceptance.py").read_text())'

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
