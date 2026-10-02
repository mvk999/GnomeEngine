# Coding Standards

- Rust source and primary technical documentation use English.
- Prefer explicit error handling for expected I/O, GStreamer, and IPC failures;
  avoid `unwrap()` in runtime event paths.
- Keep modules cohesive and lifecycle ownership visible.
- Use standard Rust, GTK/GLib, and GStreamer APIs before adding abstractions or
  dependencies.
- New compiler and Clippy warnings fail CI. Do not suppress a warning without
  recording why the exception is necessary.
- Comments explain constraints, lifecycle, or non-obvious decisions, not the
  syntax of the next line.
- Keep UI-facing error messages readable and technical diagnostics in logs.
- Do not add dead code or `allow(dead_code)` for speculative future features.

Formatting and lint conventions are enforced by `./scripts/check.sh`.
