# GnomeEngine Agent Guide

## Mission

GnomeEngine is an experimental live wallpaper engine for GNOME on Wayland. The
wallpaper is secondary to the user's workflow: a running wallpaper must not
make the desktop feel heavier.

Priority order:

1. GNOME stability
2. User workflow
3. Performance
4. Correctness
5. Maintainability
6. UX
7. Visual effects
8. Feature count

The current repository contains a Rust/GStreamer rendering proof of concept.
It opens a normal GTK window; it is not a desktop wallpaper application.

## Read Before Coding

Read [ARCHITECTURE.md](ARCHITECTURE.md), then:

- [Product vision](docs/product/vision.md)
- [Engineering principles](docs/engineering/principles.md)
- [Testing strategy](docs/engineering/testing.md)
- [Performance practice](docs/engineering/performance.md)
- Relevant [design notes](docs/design/)
- Accepted [architecture decisions](docs/decisions/)
- Active [execution plans](docs/plans/active/)

## Development Style

Use small, reviewable changes; TDD for separable domain behavior; regression
tests for bugs; continuous refactoring; and documentation alongside behavior
or architecture changes. Do not build speculative abstractions or combine
unrelated feature, refactor, and formatting work.

Before changing code, inspect the current implementation and tests, identify
the smallest coherent slice, and consider its lifecycle, performance, and
security effects. Prefer existing patterns and dependencies. Ask only when a
material product or architecture decision cannot be resolved from the code and
documented constraints.

## Validation

Run `./scripts/check.sh` before considering code complete. It is the canonical
fast check used locally and by CI. Record unavailable tools or environment
limits accurately; never describe an unrun check as passing. GNOME graphical
integration remains a separate manual validation task; see
[manual testing](docs/engineering/manual-testing.md).

## Performance and GNOME Safety

Performance is a functional requirement. Prefer events to polling, lazy work to
eager work, and resource release when playback is hidden or stopped. Do not
claim low CPU, hardware decoding, or zero-copy without measurements.

GNOME Shell extensions execute inside the Shell. Keep them small and reversible;
video decoding, rendering, media processing, and heavy I/O belong outside the
Shell. GNOME Shell stability takes priority over GnomeEngine behavior.

## Security

Treat manifests and future wallpaper packages as untrusted input. Validate
paths and sizes, avoid shell command construction, and keep D-Bus methods
minimal. A local file is not automatically trusted. See
[security baseline](docs/engineering/security.md).

## Plans and Decisions

For work spanning components or changing architecture, create an ExecPlan under
`docs/plans/active/`; keep progress and discoveries current, then move it to
`docs/plans/completed/`. Record durable architectural trade-offs as an ADR in
`docs/decisions/`. Track only consequential deferred work in
`docs/plans/tech-debt.md`.

## Commits

Use small semantic commits (`feat:`, `fix:`, `test:`, `refactor:`, `docs:`,
`ci:`, `build:`, `perf:`, `chore:`). Each commit should be coherent and leave
the repository in a valid state. Do not rewrite `main`, force-push, hide
warnings, or remove failing tests without understanding the cause.

## Never

- Implement a feature by deleting and replacing the working prototype without
  a demonstrated technical need.
- Add permanent polling, a helper process, or a large dependency without a
  measured or concrete reason.
- Put decoding or heavy rendering work in GNOME Shell.
- Claim behavior, validation, or performance that was not observed.
- Implement future platforms or systems without a product decision.
