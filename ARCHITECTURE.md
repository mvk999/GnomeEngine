# Architecture Map

## Current state by milestone

M1 background placement remains incomplete: `renderer/` still presents the
GStreamer paintable in an ordinary GTK window, not behind desktop icons or on
the GNOME background layer. The extension does not place or control that
surface.

M2's session-bus renderer service is implemented. It owns one D-Bus name and
exports `ApplyVideo`, `Pause`, `Resume`, `Stop`, and `GetStatus`. M3 adds an
independent pause-reason set, event-driven shell lifecycle updates, and
best-effort system-bus observers for UPower and logind. M4 adds a native
single-instance GTK4/Libadwaita app and local managed-copy library, with preview
and renderer control through a signal-driven D-Bus client.

```text
GnomeEngine app -- local library / GTK preview
        |
        | session D-Bus control/status
        v
Local video -> GStreamer playbin -> GTK paintable -> preview window
                                      ^
                                      |
                         renderer service
                                      ^
GNOME Shell extension -- lifecycle --+
UPower / logind -------- system bus -+
```

These service and lifecycle layers control the prototype's playback, but do
not turn its preview window into a real desktop background.

## Intended background process boundary

The following remains a target design, not implemented functionality:

```text
GnomeEngine desktop app
        | D-Bus control and status
        v
External renderer service
        | renderer surface / compositor integration
        v
Minimal GNOME Shell integration -> desktop background layer
```

The GUI can exit while the renderer remains active. The renderer must remain
outside GNOME Shell so media work cannot stall or crash the Shell. D-Bus is for
control and status, never video frames. The compositor surface-sharing and
background-layer mechanism remains an open design task, so current Apply
starts the renderer prototype's ordinary GTK preview window rather than a real
desktop wallpaper.

## Planned renderer types

| Component | State | Responsibility |
| --- | --- | --- |
| Video renderer | Prototype | GStreamer demux, decode, timing, and GTK paintable output |
| Shader renderer | Planned | GPU shader playback, loaded only for an active shader wallpaper |
| Web renderer | Planned | Isolated web content, loaded only for an active web wallpaper |

Only video playback in a normal GTK window exists today. The app and a local
library now exist; there is no online catalog or native installation package.
The Shell extension is lifecycle-only and must remain a small, reversible
controller.

## Detailed references

- [Product vision](docs/product/vision.md)
- [Renderer design](docs/design/renderer.md)
- [Desktop application design](docs/design/desktop-application.md)
- [Local wallpaper library format](docs/design/wallpaper-library.md)
- [GNOME integration design](docs/design/gnome-integration.md)
- [Lifecycle design](docs/design/lifecycle.md)
- [Architecture decisions](docs/decisions/README.md)
- [Performance record](docs/performance.md)
