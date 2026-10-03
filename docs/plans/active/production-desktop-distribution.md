# M8: Production Desktop Distribution

## Goal

Turn the existing Ubuntu `.deb` into a dependable desktop-install artifact:
correct application identity, Debian dependency resolution, standard GNOME
launcher integration, predictable artifacts, install/upgrade/remove safety,
and CI validation. Do not claim platform support or publish a public release
until the M7 runtime acceptance gates pass.

## Current packaging state

- M5 already uses debhelper and installs the GUI, external renderer, desktop
  file, AppStream component, hicolor icon, session D-Bus service, and GNOME
  Shell extension under its actual UUID.
- The build uses the Cargo lockfile and creates `dist/`; it does not install
  files, use sudo, enable the extension, or run custom maintainer scripts.
- `dh_shlibdeps` supplies ELF-derived dependencies. Runtime GStreamer plugin
  dependencies are explicit. The GTK4 paintable sink is statically registered
  by the renderer; system GTK/GStreamer libraries are not bundled.
- There is no GSettings schema: current renderer preferences use its XDG
  configuration file. User library/configuration live outside package paths.
- Application ID is `io.github.mvk999.GnomeEngine`; activation presents the
  existing active window. The desktop filename/icon basename and Wayland app
  identity already agree. A running X11 build reported `WM_CLASS` instance and
  class `gnomeengine`; this is the measured value for `StartupWMClass`.
- Previous local `.deb` builds were inspected by the user, but no installed
  package, launcher/Dock flow, upgrade, or uninstall has yet been validated.
- The worktree contains pre-existing M7, battery-settings, and GUI changes.
  They are preserved and are included in the source audit; this plan does not
  treat them as M8 changes.

## Target platforms

| Intended runtime | Current status | Packaging consequence |
| --- | --- | --- |
| Ubuntu 24.04 / GNOME 46 / Wayland | M7 in progress; only targeted nested Apply/Stop smoke noted | Do not declare supported or add Shell 46 metadata yet |
| Ubuntu 24.04 / GNOME 46 / X11 | M7 in progress; targeted real-session EWMH Apply/Stop smoke noted | Do not declare supported or publish an installable Noble target yet |
| Ubuntu 26.04 / GNOME 50 / Wayland | M7 runtime regression not performed | Current extension metadata and package dependency remain Shell 50; public release blocked |

The 24.04 host has GTK 4.14.5, Libadwaita 1.5.0, GStreamer 1.24.2, GNOME
Shell 46, and amd64. CI runner availability is verified for both
`ubuntu-24.04` and `ubuntu-26.04`; runner availability is not runtime product
validation.

## Application identity

- Keep `io.github.mvk999.GnomeEngine` as the GTK/GApplication ID, Wayland app
  ID, desktop ID basename, icon basename, and AppStream component ID.
- Keep the renderer separately identified as
  `io.github.mvk999.GnomeEngine.Renderer`; it is a background service, not a
  launcher or second normal desktop application.
- Keep GApplication single-instance activation. The activation handler presents
  the existing window when it is already open.
- For X11, use `StartupWMClass=gnomeengine`, based on the observed
  `WM_CLASS(STRING) = "gnomeengine", "gnomeengine"`; confirm association from
  an installed launcher before calling Dock behavior validated.

## Desktop integration

- Install exactly `/usr/share/applications/io.github.mvk999.GnomeEngine.desktop`
  and the matching hicolor scalable icon.
- Do not modify GNOME favorites, user dconf, enabled extensions, or home
  directories during installation.
- Retain native GApplication activation and do not add a separate renderer
  desktop entry.
- Install the system-wide extension at its UUID directory, but leave it
  disabled. The GUI's existing integration status/action remains responsible
  for user-consented enablement.
- Wayland launcher association is identity-aligned in metadata; Dock/overview
  behavior remains a manual acceptance item. Renderer visibility and desktop
  placement are M7 gates, not inferred from a successful package build.

## Runtime dependencies

- Use `${shlibs:Depends}` from debhelper for ELF-linked libraries and
  `${misc:Depends}` for helper-generated dependencies.
- Keep GNOME Shell `>= 50` while packaged extension metadata targets only 50.
  This deliberately means this candidate is not an Ubuntu 24.04 installable
  target yet; widening it before M7 runtime validation would be misleading.
- Depend on the distribution GTK GStreamer media backend and GStreamer
  playback/demux/H.264 decoder plugin packages required for the MVP. Do not
  depend on `gstreamer1.0-gtk4` or the Noble-unavailable packaged paintable
  sink; the upstream sink registration is private/static in the renderer.
- Do not bundle GTK, GLib, Libadwaita, GStreamer core, libc, Mesa, or GPU
  drivers. Do not expose build tools as runtime dependencies.
- No `Recommends` is used for packages required by the declared MP4/H.264 MVP
  path. Broader codecs remain optional/not promised.

## Private bundled components

The only private runtime component is upstream `gst-plugin-gtk4` 0.13.0
statically linked/registered inside the renderer. It is not installed into a
global GStreamer plugin directory and adds no process or environment setting.
The MPL-2.0 notice and source/version information are retained in package docs.
System GTK/GStreamer dependencies remain managed by APT.

## Package layout

Keep M5's layout: `/usr/bin/gnomeengine`, `/usr/libexec/gnomeengine/` renderer,
desktop entry, icon, AppStream file, D-Bus activation descriptor, UUID-matched
extension files, logo resource, and third-party license notices. No GSettings
schema is currently applicable. Package no user data and no repository/build
tree.

## Build strategy

- Build amd64 binaries/package from the single source tree and locked Rust
  dependency graph.
- Prefer the Noble build environment as the oldest library ABI baseline. It
  may produce only a package for the currently declared Shell 50 target; do
  not label it as a Noble runtime package.
- Emit a deterministic target-named candidate under `dist/`, plus
  `SHA256SUMS`; inspect ELF dependencies, control fields, and package contents.
- Validate on the 26.04 runner that the Noble-built package's dependency set is
  satisfiable. A second build on 26.04 is a compatibility check, not a separate
  target artifact unless a demonstrated dependency difference requires it.
- Use `dpkg-shlibdeps` through debhelper, `desktop-file-validate`, AppStream,
  extension metadata/syntax checks, `dpkg-deb`, and `lintian` when available.

## Installation, upgrade, and removal

The eventual public instructions are `sudo apt install ./<package>.deb`, which
lets APT resolve the declared dependencies. Do not recommend raw `dpkg -i` as
the primary path. The build script remains unprivileged and never installs.
No postinst script launches the app or changes a user's shell state. Removal
removes package-owned files only; upgrade/reinstall must leave XDG library and
settings untouched. These require an installed-package manual test and remain
open.

## Dock/taskbar integration

The one GUI application owns the stable desktop identity and presents its
existing window on repeated activation. The renderer has a distinct service
ID and no desktop entry. Validate app grid/search, Alt+Tab, Overview, Dock icon,
re-activation, and pin/unpin manually on actual target sessions. Do not alter
favorites automatically. Wayland and X11 launcher association are distinct
manual checks.

## D-Bus activation

Keep the existing renderer session service descriptor for modern activation.
GNOME 46/Wayland's Shell-owned renderer startup seam is an M7-specific
exception; packaging must include the extension's coordination code and must
not force the app to start another renderer. Do not add a daemon or root
service. Verify Apply/Stop after installation on each target session.

## GNOME extension installation

Install only the UUID-scoped runtime JavaScript and metadata. Do not install
TypeScript sources, `node_modules`, tests, or build tools. Do not auto-enable
the extension as root. The current `shell-version` remains `50` until M7's
GNOME 46 runtime validation actually passes.

## GSettings

No schema is used by current code. Renderer lifecycle preferences persist in
XDG config; the package must not create global compiled schema caches or
invent a schema.

## CI artifacts and GitHub Releases

- CI runs canonical checks and package validation on Noble and Resolute.
- Upload short-lived validation artifacts; do not publish a public release for
  every push.
- A tag-triggered public release is gated on explicit repository-level runtime
  validation approval after M7 acceptance. Until then, no release job may
  publish artifacts as supported downloads.
- Once unblocked, attach the Noble-built candidate and SHA256 manifest; only
  add a separate Resolute artifact if real dependency/package differences
  justify it.

## Validation matrix

| Check | Noble | Resolute |
| --- | --- | --- |
| Fast Rust/extension checks | Required | Required or package compatibility job |
| Build package | Required oldest ABI baseline | Build/compatibility check |
| APT dependency resolution | Manual/isolated install pending | Verify Noble artifact dependencies |
| GUI install/launcher/Dock | Manual, actual GNOME 46 session pending | Manual GNOME 50 Wayland pending |
| Apply/Stop/lifecycle | M7 full matrix pending | M7 regression pending |
| Upgrade/remove/user-data preservation | Manual pending | Manual pending |

## Acceptance criteria

- [x] Existing package, launcher, extension, D-Bus, version, and CI state
  audited; canonical check is green at audit start.
- [x] Desktop entry/AppStream/icon identity and measured X11 WM class hint are
  aligned.
- [x] The GUI offers an explicit, user-initiated extension enable action when
  the packaged extension files are present; package installation itself does
  not change enabled extensions.
- [x] Package dependency metadata declares the current Shell 50 floor, GTK
  media backend, GStreamer base/good/bad/libav path, and generated ELF
  dependencies. Actual H.264 playback validation remains a target-runtime gate.
- [x] Build script validates package fields/layout, creates the target-named
  `dist/` artifact and checksum without sudo, and refuses to replace a
  mismatched package at that artifact path.
- [ ] CI builds from Noble and checks that the same ABI-baseline package has
  satisfiable dependencies on Resolute; artifacts are short-lived.
- [x] README uses `apt install ./...deb` for eventual user installs and clearly
  says no public, runtime-validated download is available yet.
- [ ] Installed launcher, app identity/single-instance, Dock, install,
  upgrade/remove, and data preservation are manually verified on runtime
  targets; no such validations are inferred from package building.
- [ ] Public release remains gated until the full M7 target matrix passes.

## Progress

- [x] Inspected git status/history, architecture, README, CI, Cargo manifests,
  app/renderer identity, Debian metadata/rules, scripts, extension, and M7 plan.
- [x] Ran `./scripts/check.sh`; it passed before M8 edits (25 app tests, 24
  renderer tests, formatting/clippy, extension syntax).
- [x] Inspected host versions: Ubuntu 24.04.5, GNOME 46 X11, GTK 4.14.5,
  Libadwaita 1.5.0, GStreamer 1.24.2, amd64.
- [x] Measured GUI window `WM_CLASS` with `xprop`: `gnomeengine`; GTK
  application ID is `io.github.mvk999.GnomeEngine`.
- [x] Correct desktop/AppStream metadata, explicit runtime dependencies,
  target artifact naming, package checks, checksum generation, CI artifact
  retention, and a tag release gate have been implemented.
- [ ] Execute CI on both hosted Ubuntu runners and verify Noble-built APT
  resolution on Resolute; workflow edits are not themselves CI evidence.
- [x] Build and inspect the current candidate package on Noble; verify
  debhelper-derived ELF dependencies, contents, sizes, and checksum.
- [ ] Install/launcher/Dock/upgrade/remove acceptance remains manual and is
  not claimed by this implementation pass.

## Discoveries

- The source app and desktop ID already use the canonical identity; no app ID
  rewrite is needed. The X11 class mismatch is known and measured.
- The current control metadata requires Shell 50 and has no `Recommends`; the
  extension declares only Shell 50. An Ubuntu 24.04 package cannot be honestly
  advertised as installable until M7 validates and widens this compatibility.
- The current build wrapper's dependency assertions are stale: it checks for
  `gstreamer1.0-gtk4` even though private static registration intentionally
  replaced that runtime package dependency.
- The host has not installed GnomeEngine from a `.deb`; existing package
  inspection by the user is not an install/Dock/upgrade test.
- No GSettings schema is needed. GUI and renderer already implement
  `--version` using the Cargo package version.
- GNOME Shell's existing `org.gnome.shell` GSettings schema can be changed by
  the explicit in-app action; the package does not ship or compile a schema.
- GitHub-hosted `ubuntu-26.04` is currently an available runner label, but CI
  availability alone does not validate GNOME runtime behavior.

## Decisions

- Keep the package's current Shell 50 floor and do not mark GNOME 46 supported
  until M7 validation and extension metadata updates are complete.
- Produce a clearly target-labelled candidate from the Noble ABI baseline;
  do not publish it while M7 runtime gates remain open.
- Keep the static GTK4 sink private to the renderer and rely on distro-managed
  system libraries/dependencies.
- Use the measured X11 class as `StartupWMClass`; leave Dock validation open.
