# Renderer Design

## Implemented prototype

`renderer/src/main.rs` initializes GStreamer, opens one local path with
`playbin`, directs video to `gtk4paintablesink`, directs audio to `fakesink`,
and displays the sink's `GdkPaintable` in `gtk::Picture`. End-of-stream seeks
to time zero. On application shutdown the pipeline is moved to `NULL`.

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
