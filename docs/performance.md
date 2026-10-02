# Performance record

Performance is a functional requirement. No claim that GnomeEngine is
lightweight is made without measurements.

## Measurement status

No playback/paused performance comparison has been collected. On 2026-10-02,
the available machine had Ubuntu 24.04, GNOME Shell 46 / Mutter 14, X11, and
working Rust/GTK/GStreamer build dependencies, but GStreamer's
`gtk4paintablesink` plugin was unavailable. Consequently no visible playback
pipeline could be measured. This is not the supported GNOME 50+ Wayland target.
The D-Bus smoke test verified idle service control only and is not a playback
performance result. See [performance engineering](engineering/performance.md)
for the measurement protocol.

## Available stopped-service sample

This is an idle/stopped renderer process measurement only, not an active
wallpaper result:

| State | Sample | Average CPU | RSS | Setup |
| --- | --- | ---: | ---: | --- |
| Stopped, D-Bus service alive | 10 s (`pidstat -u -r -p`, 1 s intervals) | 0.00% | 17,872 KiB | Ubuntu 24.04, GNOME 46 / Mutter 14, X11, 12 logical CPUs; no pipeline/media |

Playing, fullscreen-paused, lock-paused, battery-paused, and display-off
measurements remain unavailable because `gtk4paintablesink` is missing. This
single stopped-state sample is environment-specific and is not a general
resource-use guarantee.

## Record template

Fill this section for each measurement run, using the same video and duration
for baseline and playback. Do not commit test video files.

| Field | Value |
| --- | --- |
| Date | 2026-10-02 (environment inspection; no media run) |
| Process | `gnomeengine-renderer` (no playback baseline captured) |
| CPU average / peak | Not measured |
| RSS memory | Not measured |
| Decoder / backend | Not measured |
| Codec / resolution / FPS | Not measured |
| CPU / GPU hardware | Not measured |
| GPU driver | Not measured |
| GNOME Shell / Mutter | GNOME Shell 46.0 / Mutter 14 (host, not target) |
| Wayland session | No; host session is X11 |
| DMA-BUF / zero-copy path | Not measured |
| Baseline comparison | Not measured |
| Notes | `gtk4paintablesink` unavailable; M1 real background integration remains incomplete |

The architectural targets are minimum practical CPU use, stable and small RAM
use, GPU work limited to decode/render needs, and event-driven state changes.
