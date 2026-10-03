# Roadmap

This roadmap describes intended milestones without dates. Status reflects the
repository as inspected on 2026-10-03.

| Milestone | Goal | Status |
| --- | --- | --- |
| M0 | Rust/GStreamer local video rendering proof of concept | Prototype exists; renderer displays an ordinary GTK preview window |
| M1 | Integrate a renderer surface into the GNOME background | Version/backend-specific bridges implemented; full desktop interaction and lifecycle acceptance remains outstanding |
| M2 | Controllable external renderer service | Implemented and unit tested; real wallpaper behavior is not established |
| M3 | Smart lifecycle and adaptive performance | Implemented in code with deterministic tests; GNOME/system runtime and performance checks remain outstanding |
| M4 | Native desktop app and local wallpaper library | App, managed-copy library, import, preview, D-Bus controls, and battery setting implemented; graphical runtime acceptance remains outstanding |
| M5 | Native Ubuntu/Debian packaging and installation | Debian package layout, metadata, build scripts, and Ubuntu 26.04 CI job implemented; target package build/install and runtime acceptance remain outstanding |
| M6 | GUI V1 / prototype-to-native product interface | In progress; branded native shell, real Library, detail, settings, and truthful Displays page implemented; visual/runtime acceptance remains open |
| M7 | Ubuntu 24.04 / GNOME 46 and X11 compatibility, retaining Ubuntu 26.04 / GNOME 50 | In progress; Noble build and targeted GNOME 46 Wayland/Xorg Apply/Stop smoke tests pass, but the full acceptance matrix and GNOME 50 regression remain unvalidated |
| M8 | Multi-monitor behavior and wallpaper assignment | Planned |
| M9 | Shader wallpapers | Planned |
| M10 | Sandboxed web wallpapers | Planned |
| M11 | Shareable wallpaper package format and customization | Planned |
| M12 | Playlists and scheduling | Planned |
| M13 | Offline-first discovery and community catalog | Planned |
| M14 | Creator tooling | Planned |
| 1.0 | Stable release after compatibility, performance, and installation criteria are met | Planned |

Milestones may be reordered when implementation evidence or GNOME API
constraints justify it. The next feature should be a small vertical slice with
tests and validation, not an attempt to implement the whole roadmap at once.
