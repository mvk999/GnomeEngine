# Packaging Direction

The first native package is an upstream Ubuntu 26.04 amd64 `.deb`, assembled
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

The package targets GNOME Shell 50+ on Wayland. The currently implemented
renderer still opens a regular GTK preview window rather than the GNOME desktop
background, so the package metadata states this limitation and no public
release should imply otherwise. Target runtime installation and integration
still require validation on Ubuntu 26.04.

Runtime dependencies are explicit in `debian/control`. The package uses distro
GTK/GStreamer libraries and plugins; it does not bundle system libraries or
build tools. Debian maintainer scripts are intentionally absent, preserving
wallpaper library/configuration on removal and avoiding mutation of any user's
desktop state.

The reproducible release build is `scripts/build-deb.sh`; validators and
package tree checks are run by `scripts/check-package.sh`. The Ubuntu 26.04
CI job builds a short-lived validation artifact and does not publish a release.

For a local target build on Ubuntu 26.04, install the build dependencies listed
in `debian/control`, then run:

```sh
./scripts/build-deb.sh
```

The resulting artifact is written to ignored `dist/`. Installation and removal
are explicit user actions; the manual acceptance checklist is in
`docs/engineering/manual-testing.md`.

Flatpak, Snap, and AppImage are out of scope until their desktop integration
constraints are designed.
