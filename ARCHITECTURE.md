# Architecture Map

## Current state by milestone

M1 now has an experimental Mutter desktop-window bridge: the renderer exports
a stable GTK application ID, and the Shell extension classifies that top-level
as a `DESKTOP` window, sizes it to the primary monitor, and keeps input
transparent. The renderer fails closed unless the extension has advertised
readiness. This is not yet runtime-validated on GNOME 50+ Wayland; the local
host is GNOME 46 on X11.

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
Local video -> GStreamer playbin -> input-transparent GTK surface
                                      ^
                                      | D-Bus lifecycle/readiness
                         renderer service
                                      ^
GNOME Shell extension -- Mutter DESKTOP classification + lifecycle --+
UPower / logind -------- system bus -+
```

Mutter owns the compositor window actor; the extension does not reparent it to
private Shell groups. The bridge and lifecycle interactions still require
manual validation on the target session.

## Intended background process boundary

The target deployment boundary is:

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
control and status, never video frames. The current bridge is experimental; if
Mutter refuses the desktop window type, the renderer stops rather than showing
an ordinary player window.

## Planned renderer types

| Component | State | Responsibility |
| --- | --- | --- |
| Video renderer | Prototype | GStreamer demux, decode, timing, and GTK paintable output |
| Shader renderer | Planned | GPU shader playback, loaded only for an active shader wallpaper |
| Web renderer | Planned | Isolated web content, loaded only for an active web wallpaper |

Only local video playback exists today; there is no online catalog. The app
and package layout exist, with target installation still unvalidated. The Shell
extension remains a small, reversible surface/lifecycle controller.

## M5 distribution layout

The upstream Ubuntu package installs the app and renderer separately. The
desktop client runs from `/usr/bin`; the renderer is activated on demand by
the user's session bus from `/usr/libexec/gnomeengine`. The system-wide GNOME
Shell extension remains explicitly disabled until the user enables it.

```text
/usr/bin/gnomeengine
        | session D-Bus
        v
session bus activation -> /usr/libexec/gnomeengine/gnomeengine-renderer
        | lifecycle D-Bus
        v
GNOME Shell extension -> GNOME lifecycle events
```

M5 packaging installs the extension but does not enable it. The package is an
Ubuntu 26.04 amd64 validation target; installation and runtime acceptance on
that platform remain outstanding.

## Detailed references

- [Product vision](docs/product/vision.md)
- [Renderer design](docs/design/renderer.md)
- [Desktop application design](docs/design/desktop-application.md)
- [Local wallpaper library format](docs/design/wallpaper-library.md)
- [GNOME integration design](docs/design/gnome-integration.md)
- [Lifecycle design](docs/design/lifecycle.md)
- [Architecture decisions](docs/decisions/README.md)
- [Performance record](docs/performance.md)
