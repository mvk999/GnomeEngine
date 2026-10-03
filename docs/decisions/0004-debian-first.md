# ADR-0004: Debian Package as First Distribution

Status: Accepted

## Context

The initial target is Ubuntu on GNOME. The product should be installable from
the desktop without requiring users to build Rust code or keep a terminal open.

## Decision

The first native distribution artifact is an upstream Debian package for
Ubuntu amd64. The M5 package layout, launcher metadata, and build workflow are
implemented; building/installing the artifact on the Ubuntu 26.04 target is
still pending validation. Flatpak, Snap, and AppImage are deferred until their
GNOME Shell integration and runtime constraints are designed.

## Consequences

Package dependencies must match the supported Ubuntu release, and package
installation/removal must preserve user wallpaper content and personal
settings. Build tooling must not install software or mutate the developer host.
