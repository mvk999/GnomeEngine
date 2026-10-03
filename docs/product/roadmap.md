# Roadmap

This roadmap describes intended milestones without dates. Status reflects the
repository as inspected on 2026-10-02.

| Milestone | Goal | Status |
| --- | --- | --- |
| M0 | Rust/GStreamer local video rendering proof of concept | Prototype exists; renderer displays an ordinary GTK preview window |
| M1 | Integrate a renderer surface into the GNOME background | Experimental Mutter desktop-window bridge implemented; GNOME 50+ Wayland runtime acceptance remains outstanding |
| M2 | Controllable external renderer service | Implemented and unit tested; real wallpaper behavior is not established |
| M3 | Smart lifecycle and adaptive performance | Implemented in code with deterministic tests; GNOME/system runtime and performance checks remain outstanding |
| M4 | Native desktop app and local wallpaper library | App, managed-copy library, import, preview, D-Bus controls, and battery setting implemented; graphical runtime acceptance remains outstanding |
| M5 | Native Ubuntu/Debian packaging and installation | Debian package layout, metadata, build scripts, and Ubuntu 26.04 CI job implemented; target package build/install and runtime acceptance remain outstanding |
| M6 | GUI V1 / prototype-to-native product interface | In progress; branded native shell, real Library, detail, settings, and truthful Displays page implemented; visual/runtime acceptance remains open |
| M7 | Multi-monitor behavior and wallpaper assignment | Planned |
| M8 | Shader wallpapers | Planned |
| M9 | Sandboxed web wallpapers | Planned |
| M10 | Shareable wallpaper package format and customization | Planned |
| M11 | Playlists and scheduling | Planned |
| M12 | Offline-first discovery and community catalog | Planned |
| M13 | Creator tooling | Planned |
| 1.0 | Stable release after compatibility, performance, and installation criteria are met | Planned |

Milestones may be reordered when implementation evidence or GNOME API
constraints justify it. The next feature should be a small vertical slice with
tests and validation, not an attempt to implement the whole roadmap at once.
