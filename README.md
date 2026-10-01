# GnomeEngine

Lightweight live wallpaper engine built specifically for GNOME and Wayland.

> GnomeEngine is currently experimental.

The first milestone is a rendering proof of concept. It targets GNOME 50+ on
Wayland and prioritizes desktop responsiveness, low idle work, and correct
resource cleanup. No performance claim is made until measurements are recorded
on supported hardware.

## Current status

The Rust renderer prototype accepts one local video file, uses GStreamer
`playbin` with `gtk4paintablesink`, suppresses audio, and loops at end of stream.
It currently presents a GTK window for validating decode and paintable output.
The GNOME Shell background-layer bridge, app UI, and D-Bus controls are not yet
implemented, so this prototype does not yet apply video as the desktop wallpaper.

## Requirements

- Linux with GNOME Shell 50 or newer
- Wayland session
- Rust and Cargo
- GTK4 development libraries
- GStreamer development libraries and the GTK4 sink plugin

Dependencies are not installed automatically.

## Build and run the renderer prototype

The current code is a renderer validation prototype. It opens a normal GTK
window so you can check whether GStreamer decodes and displays a local video.
It does **not** set the video as the GNOME desktop wallpaper yet.

### 1. Check the session and tools

Run these commands in a Linux terminal:

```sh
gnome-shell --version
echo "$XDG_SESSION_TYPE"
rustc --version
cargo --version
pkg-config --modversion gtk4
pkg-config --modversion gstreamer-1.0
gst-launch-1.0 --version
```

The target is GNOME Shell 50+ in a Wayland session. `XDG_SESSION_TYPE` should
print `wayland`.

### 2. Install build and runtime dependencies

On Ubuntu, the package names are:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  gstreamer1.0-tools gstreamer1.0-gtk4 gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good
```

The `gtk4paintablesink` element comes from the GTK4 GStreamer plugin. Confirm
that GStreamer can find it before building:

```sh
gst-inspect-1.0 gtk4paintablesink
gst-inspect-1.0 playbin
```

Additional GStreamer decoder plugins may be needed for the codecs in your
video. The renderer does not install codecs or other dependencies itself.
Ubuntu publishes the GTK4 sink plugin as [`gstreamer1.0-gtk4`](https://packages.ubuntu.com/search?keywords=gstreamer1.0-gtk4).

### 3. Build

```sh
cargo check --workspace
```

### 4. Play a local video

Pass an existing video file by absolute path:

```sh
cargo run -p gnomeengine-renderer -- "$HOME/Videos/wallpaper.mp4"
```

The video opens in a GTK window and loops when it reaches the end. Audio is
discarded. Close the window or press `Ctrl+C` in the terminal to stop playback.
The renderer uses GStreamer autoplugging for decoder selection. The selected
decoder and zero-copy path have not yet been instrumented, so hardware
acceleration must not be assumed.

## Architecture

- `renderer/`: external Rust/GStreamer playback process.
- `app/` (planned): GTK4 and Libadwaita configuration UI.
- `extension/` (planned): minimal GNOME Shell integration and lifecycle hooks.

See [architecture decisions](docs/architecture.md) and the
[performance record](docs/performance.md).

## Limitations

- This is a decode/render prototype, not a desktop wallpaper yet.
- No app UI, D-Bus control plane, Shell extension, fullscreen pause, or
  lock/unlock validation is included yet.
- Hardware decoding, DMA-BUF import, zero-copy behavior, and resource usage
  have not been measured.
- Only local video files are accepted; there are no downloads or scripts.

## Short roadmap

1. M0: connect the external renderer to the GNOME background layer and validate
   desktop interaction and session lifecycle.
2. Record a reproducible CPU/RAM/decode baseline.
3. M1: add the minimal GTK4/Libadwaita app and D-Bus controls.

The project license decision is pending.
