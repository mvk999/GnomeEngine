# ADR-0004: Debian Package as First Distribution

Status: Accepted

## Context

The initial target is Ubuntu on GNOME. The product should be installable from
the desktop without requiring users to build Rust code or keep a terminal open.

## Decision

The first planned native distribution artifact is a Debian package for Ubuntu
amd64. The package is not implemented yet. Flatpak, Snap, and AppImage are
deferred until their GNOME Shell integration and runtime constraints are
designed.

## Consequences

Package dependencies must match the supported Ubuntu release, and package
installation/removal must preserve user wallpaper content and personal
settings. Build tooling must not install software or mutate the developer host.
