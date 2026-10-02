# Architecture Map

## Current state

The only executable component is `renderer/`, a Rust binary that uses
GStreamer `playbin` and `gtk4paintablesink`. It accepts a local video, drops
audio, loops on end-of-stream, and presents the resulting paintable in an
ordinary GTK window. This validates local playback only; it does not set the
GNOME desktop background.

```text
Local video -> GStreamer playbin -> gtk4paintablesink -> GTK preview window
```

## Intended process boundary

The following is a target design, not implemented functionality:

```text
GnomeEngine desktop app
        | D-Bus control and status
        v
External renderer service
        | renderer surface / compositor integration
        v
Minimal GNOME Shell integration -> desktop background layer
```

The GUI must be able to exit while an applied wallpaper remains active. The
renderer must remain outside GNOME Shell so media work cannot stall or crash
the Shell. D-Bus is for control and status, never video frames. The compositor
surface-sharing and background-layer mechanism remains an open design task.

## Planned renderer types

| Component | State | Responsibility |
| --- | --- | --- |
| Video renderer | Prototype | GStreamer demux, decode, timing, and GTK paintable output |
| Shader renderer | Planned | GPU shader playback, loaded only for an active shader wallpaper |
| Web renderer | Planned | Isolated web content, loaded only for an active web wallpaper |

Only video playback in a normal GTK window exists today. The app, renderer
service, Shell extension, D-Bus control plane, library, and packages are not
implemented.

## Detailed references

- [Product vision](docs/product/vision.md)
- [Renderer design](docs/design/renderer.md)
- [GNOME integration design](docs/design/gnome-integration.md)
- [Architecture decisions](docs/decisions/README.md)
- [Performance record](docs/performance.md)
