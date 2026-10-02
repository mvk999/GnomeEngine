# Testing Strategy

## Test layers

- **Fast checks:** formatting, Clippy, and workspace unit tests via
  `./scripts/check.sh`.
- **Integration checks:** renderer/GStreamer and D-Bus lifecycle once those
  components exist and can be exercised deterministically.
- **Manual GNOME checks:** compositor integration and session behavior in a
  supported GNOME Wayland session; see
  [manual testing](manual-testing.md).

Keep the per-change suite fast. Do not require a graphical desktop for every
unit-level change.

## TDD and regressions

Use a failing test first for domain behavior when practical. Every reproducible
bug should gain a regression test when technically reasonable. Tests should
assert behavior, not incidental call counts or private implementation details.

Prioritize manifest/path validation, playback state and pause reasons, library
behavior, monitor mapping, D-Bus input validation, and lifecycle transitions as
these areas are implemented.

## Current baseline

The current renderer is one GTK/GStreamer executable with no unit or
integration tests. Its behavior is tightly coupled to GTK activation and a live
GStreamer pipeline. No tests are added solely to create an appearance of
coverage; extract a small testable unit when a real behavioral change needs it.

Coverage is diagnostic only. Do not set an arbitrary threshold before a useful
baseline exists. Document important untested behavior and use realistic
integration or manual validation where unit tests cannot represent it.
