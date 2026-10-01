# Architecture

## Current prototype

The first code slice is an external Rust process using GStreamer `playbin` and
`gtk4paintablesink`. Playback stays outside GNOME Shell, audio is discarded, and
GStreamer handles format parsing, decoder selection, and frame pacing.

The sink exposes a `GdkPaintable` for a GTK widget. That makes it useful for
validating local playback, but it does not itself attach frames to Mutter's
desktop background layer. The prototype currently opens a normal GTK window;
it must not be described or shipped as an applied wallpaper.

## Decisions

### Keep decoding outside GNOME Shell

**Decision:** Run GStreamer in an external renderer process.

**Reason:** GNOME Shell extensions execute inside the Shell process. A decoder
or heavy media pipeline there would put Shell responsiveness and stability at
risk.

**Alternative considered:** Decode video in the extension and paint it from a
Shell actor.

**Rejected because:** It places the heaviest and least predictable work inside
the desktop shell.

### Validate GStreamer output before the Shell bridge

**Decision:** Start with `playbin` and `gtk4paintablesink` as a narrow renderer
prototype.

**Reason:** GStreamer owns demuxing, decoder autoplugging, and playback timing;
the GTK sink gives the prototype a paintable without writing a decoder.

**Limitation:** A separate GNOME Shell integration step is still required to
display the output as a background while keeping the renderer out of normal
window interaction.

## Planned process boundary

```text
GTK4/Libadwaita app -- D-Bus control --> renderer process
                                          |
                                          +-- GNOME Shell integration
```

D-Bus is reserved for control and status operations. Video frames will not be
sent through D-Bus. Any future frame-sharing mechanism must be evaluated for
Mutter/Wayland compatibility, lock/unlock behavior, teardown correctness, and
CPU/GPU copies before it is adopted.
