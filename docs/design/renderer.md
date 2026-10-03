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

This is a single-process GTK playback window, not the planned long-lived
renderer service. It has no D-Bus API, explicit service state machine,
wallpaper library lookup, or Shell surface handoff.

## Intended service boundary

The future renderer owns media pipeline lifecycle and reports coarse status
transitions. The app requests operations over D-Bus. The Shell extension should
receive only surface/monitor identity and visibility decisions; decoding,
thumbnail generation, library scanning, and heavy I/O remain outside Shell.

Pause decisions must compose independent reasons so clearing fullscreen does
not resume playback while another constraint, such as manual pause, remains.
This behavior should be tested as domain logic before Shell event wiring.

## Invariants

- At most one active pipeline per unique wallpaper source unless later
  measurements and monitor requirements justify otherwise.
- Audio is discarded in the current prototype.
- GStreamer owns decode selection and frame pacing.
- Pipeline teardown reaches `NULL` before release.
- Status changes are event-driven and never emitted per frame.

See [performance practice](../engineering/performance.md) and the
[performance record](../performance.md).
