# Architecture Map

## Current state by milestone

M1 has experimental desktop-window bridges with a shared renderer and D-Bus
contract: GNOME 49+/Wayland uses `Meta.Window` classification, GNOME 46/Wayland
uses an extension-owned `Meta.WaylandClient`, and GNOME 46/X11 uses the
renderer-created GTK X11 surface with EWMH desktop hints. Every path fails
closed unless the Shell extension advertises readiness. GNOME 46 nested
Wayland Apply/Stop and GNOME 46/X11 renderer/EWMH smoke checks have been
recorded, but neither is full desktop acceptance. See the active M7 plan for
the testing gates.

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
GNOME Shell extension -- version/backend desktop bridge + lifecycle --+
UPower / logind -------- system bus -+
```

The renderer core, GStreamer pipeline, renderer-control D-Bus API, and
lifecycle are shared. Only surface ownership/classification differs by Shell
generation and display backend. Mutter owns the compositor window actor; the
extension does not reparent it to private Shell groups. The bridges and
lifecycle interactions still require manual validation on each target session.
All renderer surfaces target the full monitor rectangle, never the panel/dock
work area; Shell UI overlays the wallpaper and the renderer reserves no space.

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
control and status, never video frames. GNOME 46/Wayland uses a small Shell
integration D-Bus coordinator to start the external renderer as a
`Meta.WaylandClient`-owned child before its renderer D-Bus name is activated;
the renderer API itself remains unchanged. On GNOME 46/X11, the renderer writes
the EWMH desktop type, sticky, skip-taskbar, skip-pager, and all-workspaces
hints on its realized GDK X11 surface before mapping it. GNOME 49+/Wayland
retains the current `Meta.Window.set_type(DESKTOP)` path. All paths fail closed
rather than leaving an ordinary player window visible.

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

M5 packaging installs the extension but does not enable it. The current
candidate metadata targets GNOME Shell 50 and is built against Noble's older
system ABI; it is not a public release until installation and the M7 runtime
acceptance on Ubuntu 26.04 are complete. See the active M8 distribution plan.

## Detailed references

- [Product vision](docs/product/vision.md)
- [Renderer design](docs/design/renderer.md)
- [Desktop application design](docs/design/desktop-application.md)
- [Local wallpaper library format](docs/design/wallpaper-library.md)
- [GNOME integration design](docs/design/gnome-integration.md)
- [Lifecycle design](docs/design/lifecycle.md)
- [Architecture decisions](docs/decisions/README.md)
- [Performance record](docs/performance.md)
