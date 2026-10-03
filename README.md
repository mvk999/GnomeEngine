# GnomeEngine

Lightweight live wallpaper engine built specifically for GNOME and Wayland.

> GnomeEngine is currently experimental.

GnomeEngine is an experimental Rust/GTK4/GStreamer project targeting GNOME 50+
on Wayland. The app, local library, renderer service, and event-driven lifecycle
controller are implemented, but the renderer still opens an ordinary GTK video
window: the GNOME desktop-background bridge (M1) is not implemented. No
performance claim is made without a recorded measurement. The project has no
decided license yet.

## Current status

The native GTK4/Libadwaita app imports local videos into an XDG-managed library,
extracts available media metadata and a cached thumbnail, previews one item,
and controls the renderer over session D-Bus. The renderer supports independent
pause reasons and lifecycle status. Apply currently opens playback in the
renderer prototype's GTK preview window; it does not set the GNOME desktop
background.

## Requirements

- Linux with GNOME Shell 50 or newer
- Wayland session
- Rust and Cargo
- GTK4 development libraries
- Libadwaita development libraries
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

Build both the app and renderer before running the app, so its on-demand
renderer launch fallback can find the sibling executable:

```sh
cargo build --workspace
cargo run -p gnomeengine
```

Close the app window to exit the controller UI; this does not stop an active
renderer. The detail preview is app-local and stops when leaving the detail
page.

The canonical fast checks used by CI are `./scripts/check.sh`. It requires
Rust/Cargo and the native development libraries listed above; see
[the testing strategy](docs/engineering/testing.md).

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

- `app/`: GTK4/Libadwaita desktop client and local wallpaper library.
- `renderer/`: external Rust/GStreamer playback and lifecycle service.
- `extension/`: minimal GNOME Shell lifecycle integration.

See the [architecture map](ARCHITECTURE.md),
[decision records](docs/decisions/README.md), and the
[performance record](docs/performance.md).

For contributor and coding-agent onboarding, start with [AGENTS.md](AGENTS.md)
and the [architecture map](ARCHITECTURE.md). The product roadmap and engineering
knowledge base live under [`docs/`](docs/).

## Limitations

- This is not yet a real desktop wallpaper. The renderer uses a regular GTK
  window until M1 compositor/background integration is completed.
- GNOME 50+ Wayland runtime behavior has not been validated on the current
  GNOME 46/X11 host. Battery, suspend, multi-monitor and graphical app flows
  also need manual validation.
- Hardware decoding, DMA-BUF import, zero-copy behavior, and resource usage
  have not been measured.
- Only local video files are accepted; there are no downloads or scripts.

## Short roadmap

1. M1: connect the external renderer to the GNOME background layer and validate
   desktop interaction and session lifecycle.
2. Complete runtime validation and measure the lifecycle/resource behavior on
   supported GNOME 50+ Wayland hardware.
3. M5: native Ubuntu/Debian packaging and installation.

The project license decision is pending.
