# Renderer Design

## Implemented prototype

The renderer prefers GStreamer's `playbin` with `gtk4paintablesink` and a
`GdkPaintable` in `gtk::Picture`, sending audio to `fakesink`. It statically
registers upstream `gst-plugin-gtk4` 0.13.0 in the renderer process when the
system has not registered that element. Its Wayland EGL, X11 EGL, X11 GLX and
GTK 4.14+ DMA-BUF paths are enabled at build time; runtime backend choice is
left to GTK/GStreamer. The plugin is MPL-2.0 and its notice is recorded in
`THIRD_PARTY_LICENSES.md`. When static registration is unavailable, the
renderer retains GTK's `GtkVideo`/`GtkMediaStream` fallback, which uses the
platform GTK media backend (for example `libgtk-4-media-gstreamer` on Ubuntu
24.04). Both paths mute audio, loop video, and preserve playback position
across pause/resume. Pipeline teardown reaches `NULL` for the direct
GStreamer path; the GTK media stream is paused and released when stopped.

The renderer is a separate long-lived process and session-bus service. It
exports the M2 playback controls, reports coarse state, and composes M3 pause
reasons. The app passes a validated managed video path; the renderer does not
read library manifests. Desktop output is implemented as an experimental
bridge: the renderer opens a non-focusable, input-transparent GTK surface, and
the Shell extension asks Mutter to classify its toplevel as a desktop window.
Apply fails closed until the extension confirms readiness. This mechanism has
not been runtime-validated on its target desktop session, so it must not be
represented as a proven desktop-wallpaper implementation yet. GNOME Shell 46
and X11 compatibility are under investigation; in particular, the current
extension uses window classification APIs introduced in Shell 49. See
[GNOME integration](gnome-integration.md) and the active
[integration plan](../plans/active/gnome-background-integration.md), plus the
active [Ubuntu 24.04 compatibility plan](../plans/active/ubuntu-24-04-compatibility.md).

## Intended service boundary

The renderer owns media pipeline lifecycle and reports coarse status
transitions. The app requests operations over D-Bus. The Shell extension
handles compositor classification and lifecycle events only. Decoding,
thumbnail generation, library scanning, and heavy I/O remain outside Shell.

Wallpaper output is sized to the full monitor geometry, including regions
reserved for panels or docks. Work-area geometry is never used for wallpaper
sizing; Shell UI overlays this desktop surface. The X11 surface does not set
EWMH strut hints and therefore reserves no desktop space.

Pause decisions compose independent reasons so clearing fullscreen does not
resume playback while another constraint, such as manual pause, remains. This
behavior is implemented and tested as domain logic.

## Invariants

- At most one active pipeline per unique wallpaper source unless later
  measurements and monitor requirements justify otherwise.
- Audio is discarded in the current prototype.
- GStreamer owns decode selection and frame pacing.
- Pipeline teardown reaches `NULL` before release.
- Status changes are event-driven and never emitted per frame.

See [performance practice](../engineering/performance.md) and the
[performance record](../performance.md).
