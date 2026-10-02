# ExecPlan: Engineering Harness

## Goal

Make repository knowledge, fast validation, CI, and incremental workflow
explicit before adding product features.

## Why

The repository is an early renderer PoC and must support disciplined,
agent-assisted development without implying product features that do not
exist.

## Current state

One Rust/GStreamer binary, two concise docs, no tests, no CI, no agent guide.
The development environment is Ubuntu 24.04 with GNOME 46 on X11 and has no
Rust/Cargo toolchain or GTK/GStreamer development pkg-config files.

## Scope

Add the agent entry point, architecture map, product/engineering/design
knowledge, decision and plan conventions, a canonical check script, CI, and
README links. Add no product feature.

## Non-goals

Desktop application, Shell extension, D-Bus service, package, performance
benchmark, or artificial tests.

## Relevant files

`AGENTS.md`, `ARCHITECTURE.md`, `README.md`, `docs/**`, `scripts/check.sh`,
`.github/workflows/ci.yml`, and `.github/pull_request_template.md`.

## Architecture impact

No runtime architecture change. Documentation distinguishes current PoC from
planned desktop components.

## Implementation slices

1. Agent guide, product and architecture map.
2. Engineering/design/decision/plan references.
3. Canonical fast checks and CI.
4. README alignment and final repository review.

## Testing strategy

Run shell syntax checks and `./scripts/check.sh`. The latter is expected to
stop with a clear prerequisite message if Rust is unavailable. Run Markdown /
YAML structural checks if corresponding tools exist. Do not claim cargo checks
passed unless they ran.

## Performance and security implications

No runtime changes. Document existing media/path and future package/D-Bus/web
risks; keep rendering requirements explicit.

## Acceptance criteria

Agent onboarding points to the knowledge base; current implementation status
is factual; local and CI validation share `scripts/check.sh`; checks and branch
status are reported accurately.

## Progress

- [x] Repository and environment inspected.
- [x] Agent/architecture/product/engineering knowledge added.
- [x] Design decisions and debt documented.
- [x] Canonical script and CI added.
- [x] Shell syntax, Markdown links, YAML structure, and diff whitespace checked.
- [x] Rust check entry point exercised; it stopped with its documented missing-toolchain error.
- [x] Final Git review completed; Rust checks remain unavailable in this environment.

## Decisions discovered

No unit tests are added because the current binary contains no isolated
behavior to test without extracting code solely for coverage. The GTK/GStreamer
prototype remains unchanged.

## Post-implementation notes

The working environment is Ubuntu 24.04 / GNOME 46 / X11, with no Rust/Cargo
toolchain or GTK/GStreamer development pkg-config files. `./scripts/check.sh`
therefore exits 127 before running Rust checks. CI installs native build
dependencies and Rust stable before invoking the same script. No runtime
behavior was changed; the only Rust edit uses `&Path` instead of `&PathBuf` to
avoid Clippy's `ptr_arg` lint under the configured warnings-as-errors check.
