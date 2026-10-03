# ADR-0006: Build the Ubuntu Artifact with Debian Packaging Tools

Status: Accepted

## Context

GnomeEngine needs an inspectable Ubuntu `.deb` for the initial amd64 target,
without requiring users to build Rust code and without mutating the build host.
The application and renderer are separate processes, and the renderer already
owns its session-bus API.

## Decision

Use the conventional `debian/` source layout with debhelper compatibility 13
and `dpkg-buildpackage` to build an upstream binary package. Install the
renderer under `/usr/libexec/gnomeengine` and activate it on demand through a
session D-Bus service definition. Keep the system-wide GNOME Shell extension
installed but disabled until the user explicitly enables it. Do not add
maintainer scripts or a systemd daemon.

The package targets Ubuntu 26.04 amd64. Cargo, AppStream project licensing,
and Debian copyright metadata use SPDX `GPL-3.0-or-later`; the canonical GPLv3
text is the root `LICENSE` file. AppStream's separate metadata-document
license is `CC0-1.0` and does not change the program license.

## Consequences

- Package assembly follows distro tooling and remains inspectable with standard
  Debian utilities.
- Runtime dependencies use Ubuntu's GTK/GStreamer packages instead of bundled
  system libraries.
- Installing the package does not launch processes, alter user settings, or
  silently enable the extension.
- M1 desktop-background placement and target runtime installation still need
  validation; building a `.deb` does not establish those behaviors.
- Public releases must not claim the prototype preview window is a real GNOME
  desktop wallpaper.
