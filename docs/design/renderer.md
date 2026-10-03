# Renderer Design

## Implemented prototype

The renderer prefers GStreamer's `playbin` with `gtk4paintablesink` and a
`GdkPaintable` in `gtk::Picture`, sending audio to `fakesink`. When that
optional plugin is unavailable, it uses GTK's `GtkVideo`/`GtkMediaStream`
backend instead. Both paths mute audio, loop video, and preserve playback
position across pause/resume. This fallback uses the GTK media backend provided
by the platform (for example `libgtk-4-media-gstreamer` on Ubuntu 24.04).
Pipeline teardown reaches `NULL` for the direct GStreamer path; the GTK media
stream is paused and released when stopped.

The renderer is a separate long-lived process and session-bus service. It
exports the M2 playback controls, reports coarse state, and composes M3 pause
reasons. The app passes a validated managed video path; the renderer does not
read library manifests. Desktop output is implemented as an experimental
bridge: the renderer opens a non-focusable, input-transparent GTK surface, and
the Shell extension asks Mutter to classify its toplevel as a desktop window.
Apply fails closed until the extension confirms readiness. This mechanism has
not been runtime-validated on its target GNOME 50+ Wayland session, so it must
not be represented as a proven desktop-wallpaper implementation yet. See
[GNOME integration](gnome-integration.md) and the active
[integration plan](../plans/active/gnome-background-integration.md).

## Intended service boundary

The renderer owns media pipeline lifecycle and reports coarse status
transitions. The app requests operations over D-Bus. The Shell extension
handles compositor classification and lifecycle events only. Decoding,
thumbnail generation, library scanning, and heavy I/O remain outside Shell.

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
