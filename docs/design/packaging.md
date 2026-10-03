# Packaging Direction

The current validation candidate is for Ubuntu 26.04 amd64, assembled
with standard Debian `debian/` metadata and debhelper. `scripts/build-deb.sh`
builds into ignored `dist/` without sudo or host installation. The package
includes the application, renderer, session-bus activation service, GNOME Shell
extension, launcher, AppStream metadata, and icon. It does not run maintainer
scripts, enable the extension, modify user settings, or remove user data.
The UI's small brand logo is installed as fixed read-only data under
`/usr/share/gnomeengine/brand/`; stylesheet rules are embedded at compile time.

The project license in Cargo and the AppStream `<project_license>` field is
SPDX `GPL-3.0-or-later`; the canonical GPLv3 text is in the repository root
`LICENSE`. AppStream's distinct `<metadata_license>` field is `CC0-1.0`: it
describes the metadata document itself, not the application code. No GSettings
schema is shipped because current policy preferences are stored by the renderer
in its XDG config.

The existing package metadata still targets GNOME Shell 50+ on Wayland. M7 is
auditing Ubuntu 24.04/Noble as a new build/runtime baseline, with a Noble-built
artifact preferred so it links against the oldest supported system ABI. The
installed desktop integration is not yet validated on GNOME 46 or GNOME 50; do
not claim either target as runtime-supported until the full manual matrix passes.

Runtime dependencies are explicit in `debian/control`. The package uses distro
GTK/GStreamer libraries and plugins; it does not bundle system libraries or
build tools. The renderer statically registers the upstream GTK4 GStreamer
sink, so the runtime package does not depend on the `gstreamer1.0-gtk4` package
(not available in Noble); it still uses distribution GTK/GStreamer libraries,
and keeps the GTK media backend as a fallback. The upstream sink is MPL-2.0 and
its notice is installed as documentation. Debian maintainer scripts are
intentionally absent, preserving
wallpaper library/configuration on removal and avoiding mutation of any user's
desktop state.

The release-candidate build is `scripts/build-deb.sh`; validators and package
tree checks are run by `scripts/check-package.sh`. Ubuntu 24.04 and 26.04 CI
build short-lived candidate artifacts; the Noble artifact is then checked for
APT dependency resolution on Resolute. A tagged public release is gated on the
explicit `GNOMEENGINE_RELEASE_RUNTIME_VALIDATED=true` repository variable and
must only be enabled after M7's manual platform matrix passes.

For a local build on Ubuntu 24.04 or 26.04, install the build dependencies
listed in `debian/control`, then run:

```sh
./scripts/build-deb.sh
```

The resulting target-labelled artifact and `SHA256SUMS` are written to ignored
`dist/`. Installation and removal are explicit user actions; the manual
acceptance checklist is in
`docs/engineering/manual-testing.md`.

Flatpak, Snap, and AppImage are out of scope until their desktop integration
constraints are designed.
