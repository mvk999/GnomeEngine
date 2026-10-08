# M7: Ubuntu 24.04 / GNOME 46 Compatibility

## Goal

Support one shared GnomeEngine codebase on exactly these required runtime
targets:

| Ubuntu | GNOME Shell | Session | Status |
| --- | ---: | --- | --- |
| 24.04 LTS | 46 | Wayland | Required; not runtime validated |
| 24.04 LTS | 46 | X11/Xorg | Required; host available, integration not validated |
| 26.04 LTS | 50 | Wayland | Required; no local runtime validation |

Compatibility changes must preserve the existing renderer, D-Bus contract,
GUI, managed library, event-driven lifecycle, and package architecture. X11 and
Wayland are distinct session backends; XWayland applications in a Wayland
session do not count as X11 validation.

## Current components

- `app/`: GTK4 0.9 / Libadwaita 0.7, GTK `v4_12`, Adwaita `v1_4`; local app
  compiles on the available Noble host.
- `renderer/`: GTK4 0.9 and GStreamer-rs 0.23; statically registers upstream
  `gst-plugin-gtk4` 0.13.0 when the factory is not already available, then
  retains GTK `GtkVideo` media playback as fallback.
- `extension/`: GNOME Shell ESM module still targeting only Shell 50 metadata.
  It selects a modern Wayland, legacy owned-Wayland-client, or X11 EWMH bridge
  while retaining shared lifecycle logic. A narrow Shell integration D-Bus
  method coordinates renderer startup only on GNOME 46 Wayland; the renderer
  control API is unchanged.
- `debian/`: one package currently requiring GNOME Shell 50; the explicit
  `gstreamer1.0-gtk4` dependency has been removed because the renderer now
  bundles its private sink registration.
- `.github/workflows/ci.yml`: fast checks run on Ubuntu 24.04; package build
  validation runs on both Ubuntu 24.04 and 26.04.

## Baseline and current compatibility assumptions

Inspected host on 2026-10-03:

```text
Ubuntu 24.04.5 LTS (Noble)
GNOME Shell 46.0
XDG_SESSION_TYPE=x11
GTK 4.14.5
Libadwaita 1.5.0
GStreamer 1.24.2
```

`./scripts/check.sh` passes here: formatting, Clippy, 23 app tests, 21 renderer
tests, and extension syntax. This proves the Rust workspace builds against the
Noble development ABI; it does not prove GNOME desktop integration. The system
does not currently have the GnomeEngine extension installed or
`gtk4paintablesink` available. GNOME 46 Wayland and GNOME 50 Wayland are not
available in this host session.

The GTK application uses `GtkFileDialog` (GTK 4.10), Adw `Breakpoint`,
`NavigationSplitView`, `ToolbarView`, `StatusPage`, and `MessageDialog` APIs
within the `v4_10` / `v1_4` features. No usage above the stated GTK 4.14 /
Libadwaita 1.5 baseline was found. Current crate versions (`gtk4` 0.9.7,
`libadwaita` 0.7.2, `gstreamer` 0.23.7) are bindings versions, not minimum
system ABI requirements.

## GNOME API audit

The extension uses ESM, `Extension.enable/disable`, `Meta.Display` window and
monitor signals, `Meta.Window.get_gtk_application_id`, fullscreen state,
geometry methods, and the `Meta.WindowType.DESKTOP` / `set_type` bridge. Its
current capability check only confirms some symbols exist; it does not prove
that Mutter accepts a desktop window on either compositor backend. Exact
GNOME 46/50 runtime behavior, including X11 window identity and stacking, is
open. Audit exact API changes over GNOME 46, 47, 48, 49, and 50 before
compatibility metadata is widened.

The GJS upgrade-guide audit found a concrete API boundary: GNOME Shell 49
added `Meta.Window.set_type`, `hide_from_window_list`, and
`show_in_window_list`, replacing equivalent ownership-based functionality in
`Meta.WaylandClient`. Local Mutter 14/GJS introspection confirms
`Meta.WaylandClient` exposes `spawnv`, `owns_window`, `make_desktop`, and
`hide_from_window_list`. Native `new_indirect` and `setup_fd` symbols exist in
the library but are not present in the GJS typelib and are therefore not a
clean extension-callable route to adopt the D-Bus-activated renderer. The
implemented, user-authorized seam is for the Shell extension to start the same
external renderer through `Meta.WaylandClient.spawnv` on GNOME 46 Wayland.
The GNOME 49 guide also records removal of `Meta.Rectangle`, which this
extension does not name directly. Other reviewed guide changes for
46/47/48/50 are not APIs used by this extension. For GNOME 46/X11, the renderer
now applies EWMH desktop type, sticky, skip-taskbar, skip-pager, and
all-workspaces properties through the GDK-owned X11 connection before map.
GNOME 49+/50 Wayland retains `Meta.Window.set_type(DESKTOP)`. No compatibility
metadata has been widened; runtime behavior remains unverified.

## GTK / Libadwaita audit

Current compile-time features remain at GTK 4.10 and Libadwaita 1.4. They fit
the stated GTK 4.14 / Libadwaita 1.5 Noble platform floor. The existing
workspace was built against the local Noble libraries by the baseline check.
Keep these feature levels unless a concrete API use requires a different
minimum; audit every UI constructor/property against Libadwaita 1.5.

## GStreamer audit

Noble provides GStreamer 1.24.2 and GTK's GStreamer backend
`libgtk-4-media-gstreamer`; `gtk4paintablesink` is absent on this host. The
package currently depends on the newer distro package `gstreamer1.0-gtk4`,
which must not be a Noble dependency. Official plugin docs describe GL paths
for Wayland EGL, X11 EGL, and X11 GLX, plus optional direct DMA-BUF rendering
when GTK 4.14+ is used. Crate 0.13.0 matches gtk-rs 0.9 / gstreamer-rs 0.23,
uses GTK 4.14 and GStreamer 1.24 interfaces for DMA-BUF, and builds with
`wayland`, `x11egl`, `x11glx`, and `dmabuf` features. It is now statically
registered privately by the renderer; the system-wide plugin path is not
modified. The GTK media backend remains a fallback. Preserve the upstream
MPL-2.0 notice in `THIRD_PARTY_LICENSES.md` and installed package docs.
Do not claim acceleration or zero-copy without runtime evidence. Keep
`GtkVideo` as a deliberate fallback if it provides a safe supported path.

## Renderer backend audit

The renderer relies on GDK's selected display and queries
`supports_input_shapes()`. It does not force a backend. The GTK surface,
GStreamer pipeline, D-Bus API, and lifecycle are shared. It logs
`XDG_SESSION_TYPE`, actual GDK display type, GTK/GStreamer versions, and the
selected video output sink once. EWMH setup is selected only when GDK reports
an X11 display; the renderer does not force XWayland in Wayland sessions.
Desktop bridges remain experimental and must be independently validated.

## Wayland integration

First runtime gate: Ubuntu 24.04 / Shell 46 / native Wayland. The extension
exports `io.github.mvk999.GnomeEngine.ShellIntegration.EnsureRenderer`; only
the legacy Wayland path uses it to start the renderer as an owned Wayland
client. Modern Wayland retains D-Bus activation. Validate app, import, preview,
sink selection, extension loading, renderer application, true desktop
placement, Alt+Tab/Overview/workspaces/input behavior, lifecycle, stop, GUI
close/reopen, and lock/suspend recovery. No such Wayland session is available
on the current host.

## X11 integration

Second runtime gate: Ubuntu 24.04 / Shell 46 / actual Xorg session where
`XDG_SESSION_TYPE=x11`. X11 and XWayland-on-Wayland are not interchangeable.
The renderer writes EWMH properties on its realized, unmapped GDK X11 surface;
the extension observes the actual window and retains lifecycle event handling.
No `wmctrl`, `xdotool`, root-window, or polling implementation is used. The
current host is a genuine X11 session, but the extension is not installed and
desktop integration has not been exercised.

## Packaging implications

Build against Noble first and use dpkg tooling to derive ELF dependencies.
Remove Noble-incompatible explicit dependencies such as `gstreamer1.0-gtk4`;
retain packages actually required by the GTK media fallback and selected sink.
The package must allow GNOME 46 and 50 only after extension runtime validation.
Evaluate one Noble-built amd64 artifact on Resolute before deciding whether
separate per-Ubuntu artifacts are needed. Never raise runtime dependencies
based on the build host if the UI/API works on the Noble floor.

## Testing matrix and CI

- Fast checks on Ubuntu 24.04 and 26.04.
- Package build on Ubuntu 24.04; test that same artifact on 24.04 and 26.04.
- Type/syntax checks on extension source on both jobs.
- Nested GNOME 46 Wayland test where the VM/graphics stack supports it.
- GNOME 46 X11 requires an actual Xorg login; record `XDG_SESSION_TYPE=x11`.
- GNOME 50 Wayland remains an independent regression gate.
- Keep graphical/lifecycle acceptance manual and explicit; CI compile is not
  proof of Mutter window stacking or real wallpaper behavior.

## Performance and security implications

Do not add frame copies, global plugin search paths, periodic window scans,
extra daemons, shell commands, root processes, or system-wide environment
changes. A private sink must be registered only in the renderer process. Keep
the extension small, capability-checked, and reversible. Record actual sink,
GL path, decoder, and DMA-BUF use before making performance claims.

## Implementation slices

1. [x] Inspect status, history, baseline, host versions, docs, manifests,
   renderer/extension code, packaging, and CI.
2. [x] Confirm Rust app/renderer compile against Noble's installed libraries.
3. [x] Add X11 EWMH desktop-surface setup before map and graphics diagnostics;
   workspace builds against Noble.
4. [x] Audit Mutter 14 introspection and implement legacy Wayland/X11 seams
   while retaining the modern Wayland path.
5. [ ] Validate whole product flow on Noble GNOME 46 Wayland (nested
   Apply/Stop smoke passed; desktop semantics and lifecycle acceptance open).
6. [x] Integrate an upstream private GTK4 sink registration.
   [ ] Runtime-test the GTK4 sink on both Wayland and X11 paths.
7. [ ] Validate the whole product flow on Noble GNOME 46 X11 (real-session
   renderer Apply/Stop and EWMH-property smoke passed; extension/lifecycle
   semantics remain open).
8. [ ] Build Noble package, derive dependencies, and test it on Noble and
   Resolute; update CI matrix.
9. [ ] Validate GNOME 50 Wayland regression, lifecycle, and performance.
10. [ ] Update support matrix and move plan to completed only after the
    required manual tests are actually performed.

## Acceptance criteria

- [x] Workspace and renderer release binary build using Noble libraries;
  complete Debian package build/installation still pending.
- [ ] GTK/Libadwaita APIs stay within Noble baseline.
- [x] GTK4 sink is built/registered without depending on an unavailable
  Noble package; runtime Wayland and X11 graphics paths remain unvalidated.
- [ ] GNOME Shell extension runs on 46 and 50 using feature detection for API
  differences, with metadata updated only after validation.
- [ ] Ubuntu 24.04 GNOME 46 Wayland passes end-to-end.
- [ ] Ubuntu 24.04 GNOME 46 actual X11 passes end-to-end.
- [ ] Ubuntu 26.04 GNOME 50 Wayland remains functional.
- [ ] Package dependencies are correct for Noble and Resolute.
- [ ] CI builds/tests Noble baseline and current Ubuntu; checks and docs pass.
- [ ] No permanent polling, additional daemon, unsafe X11 helper, or false
  performance claim is introduced.

## Progress

- [ ] Finalization audit on 2026-10-08: checkout was clean at `70c4a86` on
  `main`; package metadata validation and `git diff --check` passed. The
  Ubuntu 24.04.5 command environment reports GNOME 46 and `XDG_SESSION_TYPE=x11`
  but has no running Shell process or accessible session manager, so it is not
  a usable GNOME X11 test session. Cargo/Rust and native development pkg-config
  files are also unavailable, so the canonical workspace check and real
  package build could not run. No runtime or package acceptance evidence was
  produced; keep every affected criterion open.
- [x] Corrected the package candidate filename to identify its build host and
  corrected the gated release notes to identify the Noble ABI baseline. This
  changes labels only; it does not decide package compatibility or open the
  release gate.

- [x] Repository baseline inspected at `dc68124`; M7 changes remain in progress.
- [x] `./scripts/check.sh`: passed (23 app tests, 21 renderer tests).
- [x] Host: Ubuntu 24.04.5 / GNOME 46 / X11; GTK 4.14.5, Libadwaita 1.5.0,
  GStreamer 1.24.2.
- [x] Removed the Noble-incompatible `gstreamer1.0-gtk4` dependency and added
  explicit, still-unvalidated GNOME46 bridge paths. Package Shell dependency
  and extension metadata remain intentionally at 50 pending runtime testing.
- [x] GNOME 46 nested Wayland smoke: extension loaded, `EnsureRenderer`
  launched the external renderer through `Meta.WaylandClient`, Apply reached
  `playing`, and Stop returned to `stopped`; bundled sink selected.
- [ ] Runtime GUI and visible desktop/Alt+Tab/Overview/workspace/input,
  lifecycle, and lock/suspend integration remain unvalidated. Nested session
  is not the full installed Ubuntu desktop acceptance test.
- [x] GNOME 46/Xorg real-session renderer Apply/Stop smoke: EWMH desktop type,
  sticky, skip-taskbar/pager, all-workspaces, and non-focusable WM_HINTS were
  observed with `xprop`.
- [x] Fixed a repeat-Apply regression found during GNOME 46/Xorg testing:
  creating/registering a new `GtkApplication` for each playback collided with
  the already-exported `org.gtk.Application` object after Stop. The renderer
  now keeps one registered GTK application for its process lifetime. The same
  H.264 wallpaper passed three D-Bus Apply/Stop cycles; the test ended in
  `stopped`. Apply status was `paused` because the host's battery-pause reason
  was active.
- [ ] GNOME Shell extension discovery and the full GNOME46/Xorg desktop
  semantics (Alt+Tab, Overview, workspaces, input, lock, suspend) remain
  unvalidated. GNOME 50 Wayland regression remains unavailable.
- [ ] Xephyr-only sink attempt was inconclusive because its X server lacked
  DRI3; it was not used as evidence for X11 desktop behavior.
- [x] `./scripts/check.sh`, `./scripts/check-package.sh`, Noble release build,
  and `git diff --check` passed after the compatibility changes.

## Discoveries

- Noble CI already runs the canonical checks, and the current local Noble host
  passes those checks against the minimum GTK/Adwaita/GStreamer generations.
- The current GTK UI API feature flags (`gtk/v4_12`, `adw/v1_4`) do not exceed
  the specified Noble system ABI.
- `gtk4paintablesink` is absent as a system plugin, but renderer dependency
  `gst-plugin-gtk4` 0.13.0 builds into the process with Wayland EGL, X11 EGL,
  X11 GLX, and GTK 4.14 DMA-BUF features. The Noble-hosted release renderer
  build passes and `ldd` reports no missing shared objects. Actual factory
  registration, sink selection, and playback on either graphics backend have
  not yet been observed at runtime; GTK media remains fallback.
- `gstreamer1.0-gtk4` was an invalid Noble package assumption and is removed
  from `debian/control`; `libgtk-4-media-gstreamer` is available in Noble.
- The renderer release binary builds against the host Noble libraries and
  `ldd` reports no unresolved libraries. This is ABI/build evidence only; it
  does not prove static plugin registration, a selected GL path, video
  playback, or desktop placement at runtime.
- GNOME 46 does not expose the `Meta.Window` classification APIs used by the
  current bridge. The upstream porting guide places their addition in GNOME
  49. Mutter 14's GJS typelib exposes `Meta.WaylandClient.spawnv()`,
  `owns_window()`, `make_desktop()`, and `hide_from_window_list()`, but not the
  native `new_indirect()`/`setup_fd()` functions. The Shell extension therefore
  starts the external renderer through the public owned-client route on GNOME
  46 Wayland only; renderer media work remains outside the Shell.
- The X11 bridge uses GDK's realized surface and writes EWMH desktop, sticky,
  skip-taskbar, skip-pager, and all-workspaces properties before map. The
  real host Xorg smoke confirmed these properties and D-Bus Apply/Stop. GTK
  rewrites `WM_HINTS` while mapping, so the renderer now reapplies the
  non-focusable hint immediately after present. Alt+Tab/workspace/lifecycle
  behavior is still open.
- On a nested GNOME 46 Wayland session, the external-child/D-Bus/Apply/Stop
  smoke succeeds and the Shell no longer crashes after avoiding Mutter's
  Wayland `move_resize_frame()` path. This does not establish desktop-layer,
  Alt+Tab, Overview, workspace, or lifecycle correctness; keep GNOME 46
  experimental and metadata unchanged.
- Current session is actual X11 (`XDG_SESSION_TYPE=x11`), but no
  `gnomeengine@mvk999.github.io` extension is installed, so this is not yet
  integration validation.
- `io.github.mvk999.GnomeEngine.ShellIntegration.EnsureRenderer` coordinates
  owned renderer startup on the GNOME46 Wayland path only. The renderer's
  ApplyVideo/Pause/Resume/Stop/GetStatus interface is unchanged; modern Wayland
  and X11 leave renderer activation external.
- GitHub Actions now has package-build jobs on both Ubuntu 24.04 and 26.04;
  this change is configured but has not yet run in CI. Package installation
  on Noble remains gated because metadata still truthfully requires GNOME 50
  until the Shell 46 bridge is solved.

## Decisions

- Treat Ubuntu 24.04 as the build ABI floor; do not increase gtk-rs/adw-rs
  feature levels without API evidence.
- The GNOME46 Wayland and X11 paths are implemented. Only a nested Wayland
  Apply/Stop smoke passed; the real desktop acceptance sequence remains open.
- Do not set extension `shell-version` to 46 until runtime testing confirms
  extension activation and lifecycle behavior on GNOME 46.
- Do not label any of the three requested runtime combinations supported based
  on host compilation alone.

## Post-implementation notes

Pending. This plan remains active until all three requested platform rows pass
the manual runtime acceptance matrix.
