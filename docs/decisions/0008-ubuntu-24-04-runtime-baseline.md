# ADR-0008: Ubuntu 24.04 as the Minimum Runtime Baseline

Status: Accepted

## Context

The project was initially constrained to a future/current GNOME 50 Wayland
release. That excludes Ubuntu 24.04 LTS, whose GNOME Shell 46, GTK 4.14,
Libadwaita 1.5, and GStreamer 1.24 provide a practical stable baseline. The
product now explicitly requires Ubuntu 24.04 GNOME 46 on both Wayland and a
real Xorg session, as well as Ubuntu 26.04 GNOME 50 on Wayland.

## Decision

Build and resolve runtime ABI dependencies against Ubuntu 24.04 where
practicable, and retain one source codebase, one renderer/D-Bus contract, and
one library/UI architecture. Detect session backend separately from the
actual GTK/GDK display backend. Use runtime feature detection for GNOME Shell
API differences; advertise a Shell version in extension metadata only after
runtime validation on that version.

The officially required test matrix is limited to:

- Ubuntu 24.04, GNOME Shell 46, Wayland.
- Ubuntu 24.04, GNOME Shell 46, actual X11/Xorg session.
- Ubuntu 26.04, GNOME Shell 50, Wayland.

## Consequences

- GTK, Libadwaita, and GStreamer Rust feature levels must not exceed APIs
  available on the Noble baseline unless a concrete requirement is documented.
- A Noble-built package is the first candidate for a common amd64 artifact;
  package metadata/dependencies must be derived and tested before choosing
  whether separate per-release packages are necessary.
- Wayland and X11 remain distinct graphics/compositor paths, but the app,
  D-Bus service, renderer core, and extension business logic remain shared.
- GNOME 47, 48, 49, GNOME 50 X11, and other distributions are not declared
  supported without testing.
- Build tests are not runtime integration evidence. The matrix remains
  “not runtime tested” until manual GNOME sessions confirm the full wallpaper
  and lifecycle behavior.
