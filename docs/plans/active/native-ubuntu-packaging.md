# M5: Native Ubuntu Packaging & Installation

## Goal

Produce a reproducible, locally buildable Ubuntu amd64 `.deb` that installs
the existing GTK application, renderer service, session D-Bus activation file,
GNOME Shell extension, desktop entry, AppStream metadata, and application icon.
The package must not enable extensions or modify per-user state during
installation/removal.

The artifact is initially for validation only. The project license has been
chosen as `GPL-3.0-or-later` and the canonical GPLv3 text is in root `LICENSE`.
Public release remains blocked until the real GNOME background integration
(M1) is implemented and tested; M5 must not present the existing renderer
preview window as an applied desktop wallpaper.

## Current components

- `app/`: GTK4/Libadwaita single-instance app, local library, import/preview,
  renderer D-Bus client. Package target executable: `gnomeengine`.
- `renderer/`: GTK/GStreamer renderer service and lifecycle observers. Package
  target executable: `gnomeengine-renderer`.
- `extension/`: already-runtime JavaScript extension, UUID
  `gnomeengine@mvk999.github.io`, metadata declares Shell 50 only. It handles
  lifecycle signals but does not integrate a background surface.
- Package metadata, data assets, D-Bus activation service, and Debian packaging
  have been added. No GSettings schema exists or is needed by current code.
- Project Cargo metadata, AppStream `<project_license>`, and Debian copyright
  metadata use SPDX `GPL-3.0-or-later`; the independent AppStream metadata
  document license is `CC0-1.0`. No third-party notices were changed.

## Current platform and target

- Official target: Ubuntu 26.04 LTS (Resolute), GNOME Shell 50+, Wayland,
  amd64.
- Available host: Ubuntu 24.04.5, amd64, GNOME Shell 46, X11, GTK 4.14.5,
  Libadwaita 1.5.0, GStreamer 1.24.2. It cannot validate the target runtime.
- Fast baseline `./scripts/check.sh` passed before M5 changes (11 app tests,
  19 renderer tests, Clippy and extension syntax checks).
- `dpkg-buildpackage`, `dpkg-deb`, desktop-file and AppStream validators are
  available; `debhelper`/`dh` are not installed on this host.

## Packaging approach considered

- **Chosen:** Debian `debian/` source layout and debhelper compatibility 13,
  built through `dpkg-buildpackage`; only normal runtime files are installed.
  The build script is an unprivileged wrapper around this packaging workflow.
- **Rejected:** bespoke staging plus `dpkg-deb --build`; it duplicates package
  assembly behavior and weakens inspection/maintainer-script conventions.
- **Rejected:** `cargo-deb` or binary bundling; neither is needed for the
  upstream Ubuntu `.deb` and neither resolves desktop integration.
- The package is an upstream binary artifact, not a Debian archive submission;
  no attempt will be made to package every Rust crate as a distro source
  package.

## Package layout

- `/usr/bin/gnomeengine`
- `/usr/libexec/gnomeengine/gnomeengine-renderer`
- `/usr/share/applications/io.github.mvk999.GnomeEngine.desktop`
- `/usr/share/metainfo/io.github.mvk999.GnomeEngine.metainfo.xml`
- `/usr/share/icons/hicolor/scalable/apps/io.github.mvk999.GnomeEngine.svg`
- `/usr/share/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service`
- `/usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/`
- No GSettings schema is currently consumed by the app or renderer. Battery
  policy is stored in renderer-owned XDG config; do not invent a schema.

## Runtime and build dependencies

Derive ELF dependencies with `dh_shlibdeps`; explicitly include runtime-loaded
GStreamer plugins for `playbin`, common demux/decoder support, the GTK4
paintable sink, and GTK's `GtkVideo` media backend. Validate Ubuntu 26.04
package names and dependency resolution on the target runner before release.
Build requires Rust/Cargo, GTK4, Libadwaita, GStreamer development packages,
pkg-config, dpkg-dev, debhelper, Node.js, and metadata validators; extension
JavaScript is already runtime-ready and needs no Node at runtime.

## Extension installation and activation

Install the existing extension system-wide at its UUID directory, but do not
enable it or mutate dconf/GSettings from root package scripts. The renderer
will get a session-bus service file for on-demand activation. The app must
reach the service over D-Bus using normal auto-start semantics; developer-only
renderer spawning remains a fallback when no installed activation service is
available. No service is started merely by package installation or app launch.

## Desktop integration and metadata

Add a regular `.desktop` launcher with `Exec=gnomeengine`, no unsupported
`DBusActivatable` claim, and a truthful AppStream component. Project license
metadata uses `GPL-3.0-or-later`; the separate AppStream document license is
`CC0-1.0`. The app is still a prototype with a renderer preview window; package
description and metadata say so plainly and must not promise an animated GNOME
desktop background.

## Versioning

Use the Rust workspace version as the software version source and validate it
against Debian changelog and AppStream release metadata. Keep app, renderer,
About dialog, package filename, and metadata aligned. The current version is
`0.1.0`; do not infer a semantic release from milestone number.

## Build pipeline

`scripts/build-deb.sh` checks prerequisites, invokes unprivileged
`dpkg-buildpackage` with the lockfile enforced, validate the resulting binary
package and file tree, and copy the artifact into ignored `dist/`. It must not
install packages, write under `/usr`, enable the extension, start processes, or
require sudo.

## CI

The canonical quick checks remain intact. A separate Ubuntu 26.04 amd64
package job uses the `ubuntu-26.04` runner, installs build tools and native
development dependencies, and builds/validates a short-lived artifact rather
than publishing a Release on every push. Public tagged release remains blocked
by the M1 and target installation/runtime validation gaps.

## Installation and removal testing

Automated checks validate desktop/AppStream/extension metadata, package
contents, architecture and dependency declarations. A target Ubuntu 26.04
install, launcher test, D-Bus activation test, extension recognition, H.264
playback, background integration, upgrade, and removal test still require a
real target installation. Removal must preserve all user library/config data.

## Security

No root daemon or custom install scripts. Do not change user settings, shell
extension enablement, home directories, or running processes. Package only
fixed executable/data files, with root-owned readable data and executable
binaries. Build from the Cargo lockfile; no shell interpolation of user paths.

## Performance impact

Package installation adds no permanent process. Session-bus activation is
on-demand. The renderer remains stopped until a client requests it. No runtime
poller, update checker, helper process, or GUI autostart is added.

## Release strategy

Build/validate a local `.deb` and CI artifact first. Do not upload a public
GitHub Release until M1 desktop-background behavior and target Ubuntu
installation pass runtime acceptance.

## Acceptance criteria

- [ ] Debian/debhelper packaging builds without sudo on an Ubuntu 26.04 amd64
      host/runner.
- [ ] The `.deb` contains app, libexec renderer, D-Bus service, desktop entry,
      AppStream data, original icon, and system-wide Shell extension.
- [ ] No development paths, `target/`, Node modules, sources, or per-user files
      are installed.
- [ ] D-Bus service activation is wired for renderer control without startup
      at install/app launch.
- [ ] Extension remains disabled unless user explicitly enables it.
- [ ] Runtime dependencies match Ubuntu 26.04 and H.264 path is represented.
- [ ] Metadata validators, package file-tree checks, `dpkg-deb` inspection, and
      `./scripts/check.sh` pass.
- [ ] Target install/upgrade/uninstall and actual desktop playback are either
      validated or explicitly left pending; no public release is claimed.

## Progress

- [x] Inspect repository status/history/remotes and run the green baseline.
- [x] Review M4 components and identify actual M1/M4 validation limits.
- [x] Research Ubuntu 26.04 package names and GitHub runner availability.
- [x] Add Debian/debhelper package layout and unprivileged build/validation
      scripts.
- [x] Add launcher, AppStream metadata, icon, and renderer D-Bus activation.
- [x] Ensure the packaged GUI uses D-Bus activation before the development
      executable fallback.
- [x] Add package-content and dependency checks plus Ubuntu 26.04 CI job.
- [ ] Build and inspect `.deb` on a machine with debhelper available.
- [x] Update architecture, packaging/install docs, roadmap, and M4/M1 known
      limitations.

## Discoveries

- The current machine is Ubuntu 24.04 GNOME 46/X11, not the product target.
- `gstreamer1.0-gtk4` exists in Ubuntu 26.04 and provides the GStreamer GTK4
  sink; Ubuntu 26.04 also publishes the base/good GStreamer plugins and
`gstreamer1.0-libav` for the H.264 decoder path. The `GtkVideo` fallback uses
the `libgtk-4-media-gstreamer` provider; it is now an explicit package
dependency alongside the GStreamer GTK4 sink.
- GitHub Actions currently provides the `ubuntu-26.04` x64 runner label.
- Current build host lacks `debhelper`/`dh`, so the actual `.deb` build cannot
  be claimed here unless that build dependency becomes available.
- A non-interactive `sudo apt-get install debhelper` attempt was blocked because
  this host requires an interactive password; no system packages were
  installed by this work.
- No GSettings schema exists; package must not invent one because lifecycle
  policy is currently renderer-owned XDG configuration.
- M1 background integration is incomplete. Installing the lifecycle-only
  extension does not turn the GTK preview into a desktop wallpaper.

## Decisions

- Use standard Debian packaging with debhelper and binary-only upstream build.
- Use the maintainer-selected `GPL-3.0-or-later` license; keep public release
  gated on M1 and target-runtime acceptance.
- Do not auto-enable extensions, persist to user dconf from maintainer scripts,
  or create custom maintainer scripts.
- Use a D-Bus activation service and make the app attempt normal D-Bus
  activation before its development-mode direct-start fallback.
- Do not package a GSettings schema that no current code reads.

## Post-implementation notes

Package metadata and CI build workflow are implemented. Local package build is
still pending because this Ubuntu 24.04 host does not have debhelper installed;
target Ubuntu 26.04 build/install and GNOME runtime acceptance also remain.
`./scripts/check-package.sh`, `./scripts/check.sh`, Cargo release build, binary
version checks, and dynamic-library resolution passed locally. The first-run
GUI flow for detecting/enabling the GNOME extension is not implemented; the
system extension remains disabled unless the user enables it in GNOME.
AppStream strict validation passes. Its optional pedantic notice is the
uppercase character in the existing application ID, which the project keeps
consistent across application, desktop file, metadata, icon, and package.
