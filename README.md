# GnomeEngine

GnomeEngine is an open-source project for using local videos as live wallpapers on GNOME. The goal is to bring more life to the desktop without getting in the way of everyday work or keeping the computer busy when the wallpaper does not need to be visible.

The project is written in Rust and uses GTK4/Libadwaita for its interface, GStreamer for video playback, and a small GNOME Shell extension to integrate the video surface with the desktop. There are no accounts, online catalogs, or server connections: the library is local, and the application is designed to work offline.

## How it works

1. Import a video into the library. GnomeEngine keeps a managed local copy and creates a thumbnail and whatever metadata it can read.
2. Open the wallpaper details to preview the video, then choose **Apply Wallpaper**.
3. The application sends the request to the renderer over D-Bus. The renderer is a separate process that plays the video with GStreamer, keeping media decoding out of GNOME Shell.
4. The GNOME extension integrates the renderer surface into the desktop layer and observes system events to apply pause policies, such as fullscreen windows, screen lock, or power conditions.
5. You can close the application window without stopping the wallpaper. To stop playback, use **Stop** in the application.

In short:

```text
GTK application ── local library and controls
       │
       └── D-Bus ── Renderer ── GStreamer ── wallpaper surface
                                              │
                                       GNOME Shell extension
```

The renderer core, playback pipeline, D-Bus control API, and lifecycle are shared. Surface integration varies by GNOME session and graphics backend because Wayland and X11 use different mechanisms.

## What exists today

- A native desktop application for importing, organizing, searching, previewing, and applying video wallpapers.
- A local library with thumbnails and available media metadata.
- An external renderer controlled over D-Bus, with muted playback and pause/resume support.
- Experimental GNOME Shell integration and lifecycle policies.
- Infrastructure for shipping wallpapers with the application package. The initial wallpaper's video and thumbnail have not been added yet, so a fresh library may be empty.

The project is experimental. Basic Apply/Stop smoke tests have been run in nested GNOME 46 Wayland and GNOME 46/X11 sessions, but these do not validate the complete desktop experience, panel behavior, workspaces, screen lock, suspend/resume, or performance. GNOME 50/Wayland regression testing is also pending. See the [compatibility report](docs/engineering/compatibility.md); no platform should be considered officially supported until full validation is complete. The [M7 acceptance procedure](docs/engineering/m7-acceptance.md) describes the real-session harness for the three required targets.

There is no stable public release yet. Package and CI artifacts are for development and validation.

## Development

To build and run the application during development, you need Rust/Cargo and the GTK4, Libadwaita, and GStreamer development libraries available on your system.

```sh
cargo build --workspace
cargo run -p gnomeengine
```

Run the project's checks with:

```sh
./scripts/check.sh
```

The application can launch without the GNOME extension, but applying a wallpaper requires the GNOME integration to be available and enabled. The compatibility report and [manual testing guide](docs/engineering/manual-testing.md) explain what has been checked and what still needs testing.

## Contributing

Contributions are welcome! You can help with bug fixes, tests, documentation, accessibility, compatibility, or improvements to the application experience.

For larger changes, open an issue first so we can discuss the proposal. To contribute code, fork the repository, create a branch for your change, run `./scripts/check.sh`, and open a Pull Request describing the problem you addressed and how you tested it. GNOME integration changes may require testing in a real desktop session; clearly state which environments you used and which have not been validated.

Start with the [architecture guide](ARCHITECTURE.md), [engineering principles](docs/engineering/principles.md), and [testing strategy](docs/engineering/testing.md). For wallpapers and other media, only submit content you created or are explicitly allowed to distribute, and include the asset's license information.

## License

GnomeEngine's code is licensed under GPL-3.0-or-later. Media files may have different licenses; refer to each asset's license information.

GnomeEngine is still under active development.
