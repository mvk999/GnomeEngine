# GnomeEngine

Lightweight live wallpaper engine built specifically for GNOME.

> GnomeEngine is currently experimental.

GnomeEngine is an experimental Rust/GTK4/GStreamer project. M7 is validating
Ubuntu 24.04/GNOME 46 on Wayland and X11, and Ubuntu 26.04/GNOME 50 on Wayland.
Targeted GNOME 46 Apply/Stop smoke tests have passed, but the desktop bridge has
not passed its full visual, lifecycle, and interaction acceptance on any
target. No performance claim is made without a recorded measurement. The
project is licensed under GPL-3.0-or-later; see [LICENSE](LICENSE).

## Current status

The native GTK4/Libadwaita app imports local videos into an XDG-managed library,
extracts available media metadata and a cached thumbnail, previews one item,
and controls the renderer over session D-Bus. The renderer supports independent
pause reasons and lifecycle status. On a supported session with the enabled
GnomeEngine Shell extension, Apply asks the session-specific bridge to place its
input-transparent renderer surface in the desktop layer. The shared renderer,
GStreamer pipeline, D-Bus control API, and lifecycle are retained across those
bridges. Surface classification remains experimental pending the full target
matrix and manual interaction/lifecycle tests.

## Validation targets (not yet support claims)

The active test matrix is Ubuntu 24.04 / GNOME 46 / Wayland, Ubuntu 24.04 /
GNOME 46 / actual Xorg, and Ubuntu 26.04 / GNOME 50 / Wayland. See the
[compatibility report](docs/engineering/compatibility.md) for the exact status;
none of these rows should be treated as fully supported until end-to-end
validation is complete.

## Installation

There is not yet a public, runtime-validated release to download. M7 desktop
integration acceptance is still in progress, so CI `.deb` files are temporary
validation artifacts and are not supported end-user downloads. When a release
is available, download the artifact for the validated Ubuntu target and install
it with APT so missing declared dependencies are resolved automatically:

```sh
sudo apt install ./gnomeengine_<version>_<target>_amd64.deb
```

Then open **GnomeEngine** from the applications menu. The package does not
automatically enable the GNOME Shell extension or modify Dock favorites; when
the installed extension is available but not active, the app offers an explicit
**Ativar integração** action.

## Build requirements

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

Compile success alone does not mean desktop wallpaper integration is
supported; check the compatibility report before testing a platform.

### 2. Install build and runtime dependencies

On Ubuntu, the package names are:

```sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev \
  libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
  libgtk-4-media-gstreamer gstreamer1.0-tools gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good
```

The renderer statically registers the upstream `gtk4paintablesink` plugin in
its own process when the system plugin is absent. GTK's `GtkVideo` media backend
remains a fallback, so Ubuntu 24.04 does not need the separately packaged
`gstreamer1.0-gtk4` plugin. The sink is built with Wayland EGL, X11 EGL, X11
GLX, and GTK 4.14 DMA-BUF features; the actual selected graphics path still
requires runtime validation. Inspect available system elements if desired:

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

To attempt desktop integration, the GnomeEngine Shell extension must be
installed and enabled. The extension must confirm it can classify the renderer
surface; otherwise Apply reports that desktop integration is unavailable and
does not open a video-player window. See the current
[compatibility matrix](docs/engineering/compatibility.md) before treating any
environment as supported.

Close the app window to exit the controller UI; this does not stop an active
renderer. The detail preview is app-local and stops when leaving the detail
page.

The canonical fast checks used by CI are `./scripts/check.sh`. It requires
Rust/Cargo and the native development libraries listed above; see
[the testing strategy](docs/engineering/testing.md).

## Build the Ubuntu package for validation

The M5 package can be built on Ubuntu 24.04 or 26.04 amd64, but has not yet
passed installed user-facing acceptance on either. With the build dependencies
listed in `debian/control` installed, run:

```sh
./scripts/build-deb.sh
```

This creates a target-labelled candidate and `dist/SHA256SUMS` without
installing anything. The current extension/package metadata target GNOME Shell
50, so the candidate is named for Ubuntu 26.04 even when built on Noble (the
older ABI baseline):

```sh
ls -lh dist/gnomeengine_*_amd64.deb dist/SHA256SUMS
```

Do not treat this build artifact as a supported release until the manual
installation, launcher/Dock, upgrade/removal, and M7 runtime checks pass.

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

- GNOME 46 nested Wayland Apply/Stop and GNOME 46/Xorg renderer EWMH Apply/Stop
  smoke tests have passed, but the full desktop integration and lifecycle
  acceptance is still open; GNOME 50/Wayland has not been regression-tested.
- Battery, fullscreen, lock, suspend, Alt+Tab, Overview, workspaces,
  multi-monitor and packaged GUI flows need manual validation on target systems.
- Hardware decoding, DMA-BUF import, zero-copy behavior, and resource usage
  have not been measured.
- Only local video files are accepted; there are no downloads or scripts.

## Short roadmap

1. M7: establish Ubuntu 24.04/GNOME 46 Wayland, GNOME 46 X11, and Ubuntu
   26.04/GNOME 50 Wayland compatibility, starting with the Noble Wayland path.
2. Validate the experimental desktop bridge and lifecycle behavior on the
   supported sessions before making user-facing support claims.
3. Measure lifecycle/resource behavior on validated hardware and sessions.

M5 packaging exists for validation. Its GNOME dependency and extension metadata
remain limited to Shell 50 until the M7 matrix has passed runtime validation;
Noble and Resolute package builds are included in CI.
