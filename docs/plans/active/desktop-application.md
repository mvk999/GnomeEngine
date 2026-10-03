# M4: Native Desktop Application & Local Wallpaper Library

## Goal

Provide a native GTK4/Libadwaita application for managing a local video
wallpaper library and controlling the existing renderer through its session
D-Bus API. Closing the UI must not stop the renderer. M4 does not claim a real
GNOME desktop background: M1's renderer-to-background surface handoff is still
unimplemented.

## Current architecture

- The Cargo workspace contains `renderer/` and `app/`; the renderer is a Rust
  GTK4/GStreamer service and still displays a normal GTK preview window.
- The renderer owns `io.github.mvk999.GnomeEngine.Renderer` on the session bus
  and exports `ApplyVideo`, `Pause`, `Resume`, `Stop`, `GetStatus`, and
  `SetPauseReason`, `SetPauseOnBattery`; state and lifecycle signals are
  event-driven.
- The Shell extension reports fullscreen, lock, and built-in panel power state.
- M1 desktop-surface integration is not implemented or runtime-validated.
- Baseline `./scripts/check.sh` passed (15 renderer tests). Host GTK is 4.14.5,
  GNOME Shell is 46/X11, and `libadwaita-1-dev` is not installed. GTK/media
  behavior therefore needs a supported graphical environment for acceptance.

## Application architecture

- Add an `app/` workspace member named `gnomeengine`, application ID
  `io.github.mvk999.GnomeEngine`, using GTK4 0.9, Libadwaita 0.7, and the
  existing GLib/GIO generation. `AdwApplication` supplies single-instance
  activation.
- Keep UI state and widgets in the application; filesystem import and the
  manifest library stay in a cohesive library module; renderer IPC stays behind
  a small async client.
- The app never owns desktop playback. Preview is a separate, lazy, app-local
  pipeline and is released when leaving detail or quitting. If Apply is used
  without a service owner, the app starts the sibling renderer executable
  directly and waits for D-Bus name ownership; no shell is invoked.
- If the renderer is absent, the app may start the sibling renderer executable
  directly (no shell) as a development/runtime fallback, then wait for its
  D-Bus name. Normal control remains D-Bus-only.

## UI structure

Library is the home and empty state. A responsive Adwaita split/navigation view
contains a static-thumbnail grid, one detail page with one muted preview, and
minimal preferences/About. It adapts to narrow windows, respects system theme,
uses keyboard actions and accessible names, and has no hidden/tray process.
Displays assignment and unsupported future product areas are excluded.

## Library data model and storage

- XDG data directory: `$XDG_DATA_HOME/gnomeengine/wallpapers` (GLib fallback
  where unset); each import receives an opaque unique ID and a managed copy.
- A schema-v1 JSON manifest stores title, timestamp, relative entry path, and
  discovered media metadata. Unknown schema versions and invalid paths are
  rejected; a broken directory is skipped without preventing other items from
  loading.
- Import stages content, metadata, thumbnail, and manifest under a controlled
  temporary directory, then renames into place. Original user content is never
  deleted. Every import produces an independent library ID.
- Startup reads manifests and cached static thumbnails only; it does not probe
  media or regenerate thumbnails.

## Import and media strategy

The GTK4 file chooser is asynchronous. GStreamer discovery validates that the
file contains a readable video stream and extracts only available values.
Potentially blocking copy/discovery/thumbnail generation runs outside the GTK
main thread, with errors delivered back to the UI. A single cached thumbnail is
generated during import; failure uses a placeholder while a valid video remains
importable. Preview is lazy, muted, single-instance, and released on navigation
away. No transcoding, network access, or per-card playback is added.

## Renderer D-Bus client

The client watches well-known-name ownership, asynchronously calls `GetStatus`,
`ApplyVideo`, `Stop`, and `SetPauseOnBattery`, and subscribes to
`StateChanged`, `PlaybackError`, `PauseReasonsChanged`, and `PolicyChanged`. It
reconstructs UI state from a fresh status on service appearance/restart. It
never polls. Apply uses the managed copy and cards are marked active only when
renderer status confirms that canonical path. Closing the app never calls
`Stop`.

## Settings strategy

Expose only already-supported lifecycle choices. M4 will persist the battery
pause preference in renderer-owned user configuration and expose it through
status/a typed D-Bus method so the setting continues to apply while the GUI is
closed. Fullscreen/lock remain automatic unless a clean shared preference
mechanism can be added without leaving the extension or renderer with stale
policy. No settings database or install-time schema mutation is planned.

## Error handling, accessibility, and tests

Expected errors (missing renderer, invalid video, unreadable/unsupported
media, broken manifest, preview failure, filesystem failure, D-Bus playback
error) are user-readable and never crash the window. Details go to logs without
paths/content being unnecessarily exposed.

Pure library rules receive unit tests for manifest/schema/path validation,
discovery, title derivation, staged import rollback, and active-path matching.
Client conversion/error mapping is tested independently where feasible. GTK,
file chooser, preview, and desktop behavior use a concise manual checklist and
must not become CI display/GPU requirements.

## Performance implications

No startup media scan, thumbnail regeneration, status timer, per-card decoder,
or hidden application process. Preview is one pipeline only while detail is
active. Closing the application must release app-owned preview and leave the
external renderer untouched. Import work runs off the GTK event path.

## Security implications

Treat selected media/manifests as untrusted. Validate regular-file input,
schema version, identifier and relative entry path; canonicalize and contain
resolved paths inside each managed wallpaper directory; reject symlink/path
escapes. Never execute a path, construct a shell command, delete the original,
or deserialize arbitrary executable content. Commit imports atomically and
clean only app-owned staging directories.

## Implementation slices

1. Create this living plan and establish GTK/Libadwaita/GStreamer binding and
   platform availability; record the M3/M1 boundary.
2. Add app crate and a single-instance AdwApplication shell with native empty
   Library view, actions, and About; update CI/system dependency declarations.
3. Add manifest model and filesystem library with pure security/validation
   tests.
4. Add asynchronous staged import, GStreamer metadata discovery and cached
   thumbnail generation; wire the empty/library grid and friendly errors.
5. Add detail navigation and one muted, on-demand preview with deterministic
   cleanup.
6. Add the renderer D-Bus client, live status/signals, Apply and Stop; verify
   closing/reopening semantics where runtime allows.
7. Add minimal persistent lifecycle preferences if supported by a shared
   renderer-owned setting; update architecture, security, performance, README,
   manual testing, and roadmap. Keep unresolved M1/runtime acceptance visible.

## Acceptance criteria

- [x] App is native Rust GTK4/Libadwaita, single-instance, and workspace-tested.
- [x] Library has an empty state, responsive static-thumbnail grid, and
  schema-v1 XDG-managed storage.
- [x] Import validates actual video content, copies without touching source,
  produces metadata and a cached thumbnail, and handles failure atomically.
- [x] Detail page runs at most one muted preview and tears it down on back.
- [x] D-Bus client reflects status and signals, applies managed paths, stops on
  explicit user action only, and reconnects without polling.
- [x] Battery preference persists and is honored without the app open, if
  exposed; no speculative setting is shown.
- [x] Close/reopen UI behavior does not stop renderer and reconstructs current
  status from D-Bus.
- [ ] Tests/checks pass. Runtime GTK/import/preview and M1 desktop tests are
  reported separately and not inferred from compilation.
- [ ] No M5 packaging or install-time global mutations are added.
- [x] Safe local removal confirms intent, stops the active managed item before
  deletion, and leaves the original source alone.

## Progress

- [x] Inspected repository status, branch/history/remotes, and M3 baseline.
- [x] Read architecture, product/engineering/design/decision notes and active
  implementation plans; inspected renderer, extension, workspace, CI, and D-Bus
  contract.
- [x] Confirmed M3 code and 15 unit tests are present; M1 background bridge is
  absent and M3 target GNOME runtime remains unvalidated.
- [x] Recorded local versions: GTK 4.14.5, GNOME Shell 46/X11; Libadwaita dev
  package is missing and cannot be installed with the available sudo credential.
- [x] Added an AdwApplication-based single-instance shell, empty Library state,
  responsive static-thumbnail grid, and detail navigation with one muted lazy
  GtkVideo preview.
- [x] Added schema-v1 manifests, XDG library discovery, regular-file/path
  containment checks, stale app-owned staging cleanup, and isolated-corruption
  handling.
- [x] Added worker-thread media discovery/copy and capped PNG thumbnail import;
  no source video is modified or deleted. Thumbnail decode failures use the
  card placeholder path.
- [x] Added app/library/parser tests; the canonical workspace check passes
  with the app in the workspace.
- [x] Added a signal-driven renderer client, GetStatus mapping, Apply/Stop,
  active-item indication, renderer-on-demand launch, and persistent battery
  pause preference owned by the renderer.
- [x] Added About, native preferences, keyboard shortcuts, confirmation before
  deleting a managed copy, and policy/preference/D-Bus contract tests.
- [x] Canonical checks pass after implementation: 11 app tests, 19 renderer
  tests, Clippy with warnings denied, rustfmt, extension syntax, and
  `cargo build --workspace`.
- [x] A private-session D-Bus smoke test exercised `GetStatus`,
  `SetPauseOnBattery`, `SetPauseReason`, status contents, and persistence under
  a temporary XDG config directory.
- [ ] Perform graphical app, import, preview, and app↔renderer client runtime
  validation where a supported display/media environment allows.
- [ ] Validate GTK runtime, import, preview, and D-Bus integration where the
  host allows.

## Discoveries

- The product roadmap is stale relative to implemented M2/M3; update it with
  repository evidence while keeping M1 marked incomplete.
- `ApplyVideo` currently opens a normal GTK window, so an app integration test
  can validate D-Bus control but cannot validate desktop wallpaper behavior.
- `libadwaita-1-dev` is not installed system-wide; headers/pkg-config metadata
  were extracted under `/tmp` for local compilation. CI installs the package.
  Host GNOME 46/X11 and missing `gtk4paintablesink` do not permit validating
  the supported GNOME 50+ Wayland desktop flow or actual renderer playback.
- A session-bus smoke test is possible without a display because renderer
  startup is lazy; the host UPower state was observable in the test session,
  while the preference write was redirected to a temporary XDG config path.

## Decisions

- Use existing GStreamer/GTK binding generations (gtk4 0.9, gstreamer 0.23,
  GLib/GIO 0.20) with their compatible Libadwaita Rust binding generation.
- Keep renderer and app as separate processes. D-Bus remains the control plane;
  preview is a separate app-owned resource.
- Store media as managed copies plus small versioned manifests rather than a
  database.
- Never label the current ordinary GTK renderer window as a GNOME wallpaper.

## Post-implementation notes

Code implementation is complete for the selected M4 scope. Keep this plan in
`active/` until graphical import/preview/app-client flows are manually
validated on a usable GTK/GStreamer environment. M1 background integration
remains incomplete and must not be hidden by M4.
