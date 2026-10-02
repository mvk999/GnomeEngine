# ADR-0002: Initial Platform Boundary

Status: Accepted

## Context

Supporting multiple compositors and display servers would add separate
integration and lifecycle paths before the GNOME experience is proven.

## Decision

The initial target is GNOME 50+ on Wayland, with Ubuntu as the primary
development/distribution platform and amd64 first.

## Consequences

The project can focus testing and integration on one shell/compositor. KDE,
other desktop environments, X11, Windows, and macOS are outside the initial
support scope. Future support requires a concrete product decision and design.
