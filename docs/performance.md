# Performance record

Performance is a functional requirement. No claim that GnomeEngine is
lightweight is made without measurements.

## Measurement status

No baseline or playback measurements have been collected yet. The environment
inspected on 2026-10-02 was Ubuntu 24.04 with GNOME Shell 46 on X11 and lacked
Rust/Cargo and GTK/GStreamer development pkg-config files. It is not the
supported GNOME 50+ Wayland validation target. No CPU, RAM, GPU, or decoder
values are reported. See
[performance engineering](engineering/performance.md) for the measurement
protocol.

## Record template

Fill this section for each measurement run, using the same video and duration
for baseline and playback. Do not commit test video files.

| Field | Value |
| --- | --- |
| Date | Not measured |
| Process | Not measured |
| CPU average / peak | Not measured |
| RSS memory | Not measured |
| Decoder / backend | Not measured |
| Codec / resolution / FPS | Not measured |
| CPU / GPU hardware | Not measured |
| GPU driver | Not measured |
| GNOME Shell / Mutter | Not measured |
| Wayland session | Not measured |
| DMA-BUF / zero-copy path | Not measured |
| Baseline comparison | Not measured |
| Notes | Renderer prototype not yet integrated into the desktop background |

The architectural targets are minimum practical CPU use, stable and small RAM
use, GPU work limited to decode/render needs, and event-driven state changes.
