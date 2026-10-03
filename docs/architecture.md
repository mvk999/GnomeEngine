# Architecture Documentation Index

Use the [root architecture map](../ARCHITECTURE.md) for current and planned
component boundaries. It distinguishes the renderer prototype from unbuilt
application, D-Bus, and GNOME Shell integration components.

Durable trade-offs are recorded once in [architecture decision records](decisions/README.md).
Current component details live in:

- [Renderer design](design/renderer.md)
- [GNOME integration design](design/gnome-integration.md)
- [Packaging direction](design/packaging.md)

The renderer prefers GStreamer `playbin` and `gtk4paintablesink`, with GTK's
`GtkVideo` media backend as a fallback when that optional sink is unavailable.
Both currently display in a normal GTK window, not as a GNOME desktop
background.
