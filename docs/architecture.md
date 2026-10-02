# Architecture Documentation Index

Use the [root architecture map](../ARCHITECTURE.md) for current and planned
component boundaries. It distinguishes the renderer prototype from unbuilt
application, D-Bus, and GNOME Shell integration components.

Durable trade-offs are recorded once in [architecture decision records](decisions/README.md).
Current component details live in:

- [Renderer design](design/renderer.md)
- [GNOME integration design](design/gnome-integration.md)
- [Packaging direction](design/packaging.md)

The original renderer PoC uses GStreamer `playbin` and `gtk4paintablesink` to
show a local video in a normal GTK window. The paintable preview is not a GNOME
desktop background.
