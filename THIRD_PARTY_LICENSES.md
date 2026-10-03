# Third-party notices

GnomeEngine statically registers the upstream Rust GStreamer GTK4 sink crate
in the renderer process:

- Crate: `gst-plugin-gtk4` 0.13.0
- Upstream: <https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs>
- Copyright: Bilal Elmoussaoui, Jordan Petridis, Sebastian Dröge
- License: Mozilla Public License 2.0 (MPL-2.0)
- License text: <https://www.mozilla.org/en-US/MPL/2.0/>

The crate is not relicensed as part of GnomeEngine. Its source-file copyright
and SPDX notices remain upstream's. Its pinned version and checksums are
recorded in `Cargo.lock`.
