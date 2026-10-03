# GnomeEngine

Lightweight live wallpaper engine built specifically for GNOME and Wayland.

> GnomeEngine is currently experimental.

GnomeEngine is an experimental Rust/GTK4/GStreamer project targeting GNOME 50+
on Wayland. The app, local library, renderer service, and event-driven lifecycle
controller are implemented. An experimental Mutter desktop-window bridge now
connects the renderer to the GNOME session, but target-session behavior is not
yet validated. No performance claim is made without a recorded measurement. The project is
licensed under GPL-3.0-or-later; see [LICENSE](LICENSE).

## Current status

The native GTK4/Libadwaita app imports local videos into an XDG-managed library,
extracts available media metadata and a cached thumbnail, previews one item,
and controls the renderer over session D-Bus. The renderer supports independent
pause reasons and lifecycle status. On a supported session with the enabled
GnomeEngine Shell extension, Apply asks Mutter to classify its input-transparent
renderer surface as a desktop window. This bridge remains experimental until
validated on GNOME 50+ Wayland.

## Requirements

- Linux with GNOME Shell 50 or newer
- Wayland session
- Rust and Cargo
- GTK4 development libraries
- Libadwaita development libraries
- GStreamer development libraries and video decoder plugins
- GTK's GStreamer media backend (`libgtk-4-media-gstreamer` on Ubuntu)

Dependencies are not installed automatically.

## Build and run the application

The renderer is activated through the app and the session D-Bus service. Apply
requires the GnomeEngine Shell extension to be enabled; without it the renderer
fails closed instead of opening a normal player window.

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
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  libgtk-4-media-gstreamer gstreamer1.0-tools gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good
```

The renderer prefers GStreamer's `gtk4paintablesink` element when installed.
When it is unavailable, it falls back to GTK's `GtkVideo` media backend, so
Ubuntu 24.04 does not need a separately packaged GTK4 sink plugin. Check the
preferred sink if desired:

```sh
gst-inspect-1.0 gtk4paintablesink
gst-inspect-1.0 playbin
```

Additional GStreamer decoder plugins may be needed for the codecs in your
video. The renderer does not install codecs or other dependencies itself.
On Ubuntu, `libgtk-4-media-gstreamer` supplies the GTK media playback backend.

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

To apply a wallpaper, use GNOME 50+ on Wayland with the GnomeEngine Shell
extension installed and enabled. The extension must confirm it can classify the
renderer surface; otherwise Apply reports that desktop integration is
unavailable and does not open a video-player window.

Close the app window to exit the controller UI; this does not stop an active
renderer. The detail preview is app-local and stops when leaving the detail
page.

The canonical fast checks used by CI are `./scripts/check.sh`. It requires
Rust/Cargo and the native development libraries listed above; see
[the testing strategy](docs/engineering/testing.md).

## Build the Ubuntu package for validation

M5 packaging targets Ubuntu 26.04 amd64 and has not yet been validated as an
installed user-facing release. On that target with the build dependencies
listed in `debian/control` installed, run:

```sh
./scripts/build-deb.sh
```

This creates `dist/gnomeengine_<version>_amd64.deb` without installing it.
For local testing, install the generated artifact explicitly with
`sudo apt install ./dist/gnomeengine_*.deb`.

## Architecture

- `app/`: GTK4/Libadwaita desktop client and local wallpaper library.
- `renderer/`: external Rust/GStreamer playback and lifecycle service.
- `extension/`: minimal GNOME Shell desktop-surface and lifecycle integration.

See the [architecture map](ARCHITECTURE.md),
[decision records](docs/decisions/README.md), and the
[performance record](docs/performance.md).

For contributor and coding-agent onboarding, start with [AGENTS.md](AGENTS.md)
and the [architecture map](ARCHITECTURE.md). The product roadmap and engineering
knowledge base live under [`docs/`](docs/).

## Limitations

- The Mutter desktop-window bridge is experimental and has not been validated
  on the GNOME 50+ Wayland target. The available host is GNOME 46 on X11.
- GNOME 50+ Wayland runtime behavior has not been validated on the current
  GNOME 46/X11 host. Battery, suspend, multi-monitor and graphical app flows
  also need manual validation.
- Hardware decoding, DMA-BUF import, zero-copy behavior, and resource usage
  have not been measured.
- Only local video files are accepted; there are no downloads or scripts.

## Short roadmap

1. Validate the experimental M1 desktop bridge and lifecycle behavior on
   supported GNOME 50+ Wayland hardware.
2. Measure lifecycle/resource behavior on
   supported GNOME 50+ Wayland hardware.
3. M5: native Ubuntu/Debian packaging and installation.

M5 packaging is implemented for validation, but still requires a target Ubuntu
26.04 package build and installation/runtime acceptance before a public release.
