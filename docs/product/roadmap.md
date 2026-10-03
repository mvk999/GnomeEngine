# Roadmap

This roadmap describes intended milestones without dates. Status reflects the
repository as inspected on 2026-10-02.

| Milestone | Goal | Status |
| --- | --- | --- |
| M0 | Rust/GStreamer local video rendering proof of concept | Prototype exists; renderer displays an ordinary GTK preview window |
| M1 | Integrate a renderer surface into the GNOME background | Not implemented; compositor/background-layer integration remains unresolved |
| M2 | Controllable external renderer service | Implemented and unit tested; real wallpaper behavior is not established |
| M3 | Smart lifecycle and adaptive performance | Implemented in code with deterministic tests; GNOME/system runtime and performance checks remain outstanding |
| M4 | Native desktop app and local wallpaper library | App, managed-copy library, import, preview, D-Bus controls, and battery setting implemented; graphical runtime acceptance remains outstanding |
| M5 | Native Ubuntu/Debian packaging and installation | Planned; not part of M4 |
| M6 | Multi-monitor behavior and wallpaper assignment | Planned |
| M7 | Shader wallpapers | Planned |
| M8 | Sandboxed web wallpapers | Planned |
| M9 | Shareable wallpaper package format and customization | Planned |
| M10 | Playlists and scheduling | Planned |
| M11 | Offline-first discovery and community catalog | Planned |
| M12 | Creator tooling | Planned |
| 1.0 | Stable release after compatibility, performance, and installation criteria are met | Planned |

Milestones may be reordered when implementation evidence or GNOME API
constraints justify it. The next feature should be a small vertical slice with
tests and validation, not an attempt to implement the whole roadmap at once.
