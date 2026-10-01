# Performance record

Performance is a functional requirement. No claim that GnomeEngine is
lightweight is made without measurements.

## Measurement status

No baseline or playback measurements have been collected yet. The development
session used for this initial code slice did not provide a usable Linux command
executor, so no GNOME, Wayland, GStreamer, CPU, RAM, or GPU values are reported.

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
