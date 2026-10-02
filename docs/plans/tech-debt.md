# Technical Debt

## GNOME background surface handoff is unresolved

- **Problem:** The current GTK paintable renderer is an ordinary window and no
  validated mechanism integrates it into the GNOME background layer.
- **Impact:** The product cannot yet apply a live wallpaper.
- **Why deferred:** The repository has only a playback PoC, and this requires
  current GNOME 50+ Wayland API research and graphical integration testing.
- **Revisit when:** A GNOME 50+ Wayland nested/dev environment is available for
  a focused bridge prototype.

## No automated tests exist

- **Problem:** The current renderer couples argument handling, GTK activation,
  and GStreamer lifecycle in one binary and has no tests.
- **Impact:** CI can enforce formatting and linting but has no behavior
  regression suite yet.
- **Why deferred:** There is no isolated domain behavior worth extracting
  solely to create tests. Extract a small unit alongside the first real state
  or input-validation change.
- **Revisit when:** Renderer state, URI preparation, manifest validation, or
  another independently testable behavior is introduced.

## Performance baseline is missing

- **Problem:** No reproducible idle/playing/paused measurements are recorded.
- **Impact:** Resource-use claims and regressions cannot yet be quantified.
- **Why deferred:** The current environment lacks Rust/GStreamer development
  tooling and does not run the supported GNOME Wayland target.
- **Revisit when:** A supported GNOME 50+ Wayland development system is
  available with a documented test video.
