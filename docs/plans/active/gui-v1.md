# M6: GUI V1 / Prototype-to-Native Product Interface

## Goal

Translate the supplied GnomeEngine HTML prototype into a polished, adaptive
Rust/GTK4/Libadwaita interface while keeping the existing library, import,
preview, renderer D-Bus, and lifecycle implementations as the sources of truth.
This milestone changes presentation and user-facing flows; it does not claim
that the renderer is a real GNOME desktop background.

## Current UI and architecture

- The complete prototype was supplied inline in the task. No local
  `gnomeengine-prototipo(1).html` file exists in the checkout. It was reviewed
  in full, including Library, empty state, detail, import modal, Displays,
  Settings, and diagnostic error states.
- `app/src/window.rs` is currently a single large window implementation using
  `AdwToolbarView`, `AdwHeaderBar`, and a `GtkStack`; the Library uses a
  manually populated `GtkFlowBox`. Most copy is English and the branding is
  absent. Cards already use actual cached thumbnails and manifest metadata.
- `app/src/library/` owns the real XDG library, safe managed imports/removals,
  metadata, and cached thumbnails. `RendererClient` owns asynchronous,
  signal-driven D-Bus status, Apply, Stop, Pause/Resume, and battery policy.
- The app currently receives no monitor inventory or display dimensions over
  D-Bus. Only the renderer's current active path, state, reasons, and battery
  preference are available. M6 must not fabricate monitor rows or a fullscreen
  preference that the backend does not expose.
- Applying still opens the renderer's ordinary GTK playback window. M1's
  desktop-background surface integration remains incomplete.

## Prototype analysis

- Calm, dark-first visual language: charcoal base/panels, subtle separators,
  low-contrast borders, restrained purple accent (`#c9a8ff`), green active
  state, compact supporting text, and large actual wallpaper imagery.
- A roughly 236 px sidebar establishes brand, short tagline, navigation, a
  compact active-wallpaper status, and GNOME/Wayland session context.
- Library hierarchy: eyebrow, `Biblioteca`, short instruction, prominent
  import action, local search/filter, responsive static thumbnail cards,
  metadata and a clear active marker; a real empty library has a welcoming
  import state.
- Detail hierarchy: back navigation, large live preview, concise actual
  metadata, Apply/Stop based on current renderer state, and an explanation that
  renderer playback is independent of the app window.
- Displays, Settings, and diagnostics use quiet grouped sections, real state,
  understandable language, and action feedback. Prototype-only artwork/data,
  demo controls, fake progress, remote Inter font, CSS effects, and mobile
  bottom navigation are not product features.

## Native translation strategy

- Keep `AdwApplication` single-instance, `GtkVideo` as one lazy muted preview,
  and `RendererClient` as the only desktop-playback controller.
- Use Libadwaita navigation/adaptive widgets supported by the checked-in Rust
  bindings and declared minimum versions; prefer native navigation behavior
  over copying browser/mobile geometry. Respect system light/dark preference.
- Use `GtkGridView`/GIO list-model-backed reusable cells if it fits the current
  model cleanly; otherwise record why a simpler existing GTK container is
  preferable for this library size. Cells remain static thumbnails.
- Extract the supplied SVG logo unchanged into a project asset and ensure both
  development and M5 package resource paths work independently of CWD.
- Keep Portuguese user-facing strings from the prototype where appropriate.
  Use real manifest metadata and renderer status only; unknown monitor,
  lifecycle, and media fields are omitted or shown as unavailable.
- Import continues through the existing async chooser and worker. Progress is
  indeterminate and shown only for real work; no timer-driven fake percentage.

## Screen and state inventory

- Library: populated, empty, title search/filter, active item, renderer
  unavailable/paused/error status.
- Detail: actual cached/preview media, available metadata only, Apply or
  active/Stop actions, preview teardown when navigating away.
- Import: native asynchronous file selection, actual ongoing work feedback,
  success/error toast, no fake files or simulated progress.
- Displays: report only actual data currently available; if there is no monitor
  inventory contract, explain the current all-displays behavior without
  inventing monitor names or geometry.
- Settings: bind the existing renderer-owned battery-pause policy; do not show
  unsupported fullscreen/startup toggles. About/diagnostics use real app,
  renderer, and session information.

## Accessibility and responsiveness

- Preserve keyboard activation/actions and accessible labels for icon-only
  controls/cards; state is conveyed by text as well as color.
- Test wide, medium, and narrow/tiled desktop windows, system dark/light styles,
  focus visibility, and navigation without a mouse.
- Native adaptive navigation and stacked detail layout are preferred over
  custom width polling or fixed mobile bottom navigation.

## Performance and security

- No remote fonts/assets/network, demo media, periodic status checks, or live
  video cards. Only the detail page owns one app-local preview; leaving it
  releases the stream. Closing the app never sends renderer Stop.
- Reuse the existing validated library/import operations. UI feedback must not
  expose raw D-Bus errors as primary copy; diagnostic details remain in logs.
- No new background process or renderer/lifecycle/backend rewrite.

## Implementation slices

1. [x] Inspect repository state, run the green baseline, read current docs and
   inspect the supplied full HTML and actual GTK/data/D-Bus implementation.
2. [x] Add branding asset and a native adaptive branded application shell.
3. [x] Redesign Library header, real empty state, search/filter, and responsive
   static wallpaper cards using actual model items.
4. [x] Redesign detail/preview and state-dependent Apply/Stop actions while
   preserving preview cleanup and renderer persistence after app close.
5. [x] Replace import toast-only feedback with honest native in-progress,
   success, and error feedback; retain existing async importer.
6. [x] Redesign Displays/Settings/diagnostics to show only available live
   information and supported controls.
7. [x] Add focused logic tests, update package resource installation and
   design/manual-testing docs, and validate available checks.

## Validation checklist

- [x] `./scripts/check.sh` passed before M6 and after the final UI changes.
- [x] No demo data, prototype-only controls, HTML/WebView, remote assets, or
  fake monitor/settings fields ship in the app.
- [ ] Empty/populated Library, real import, detail preview, Apply/Stop,
  renderer loss/recovery, battery settings, Displays, and About reviewed.
- [ ] Wide/medium/narrow and dark/light interfaces reviewed in a graphical
  session when available; keyboard/accessibility labels checked.
- [x] Preview is stopped when leaving detail; Library starts no media
  pipelines; app close leaves renderer untouched.
- [x] M5 package resource tree/static checks remain valid.
- [ ] Build/install the `.deb` and inspect the installed resources in a target
  environment; `debhelper` is unavailable on this host.

## Progress

- Final `./scripts/check.sh` passed: 13 app tests, 19 renderer tests, GNOME
  extension syntax, formatting, and Clippy. `cargo build --workspace`,
  `./scripts/check-package.sh`, AppStream validation, and `git diff --check`
  also passed.
- M6 now uses a native Libadwaita `NavigationSplitView` and a `GtkGridView`
  backed by a filtered GIO list model. The library cards remain static cached
  thumbnails; detail owns the only GUI preview pipeline.
- Added the prototype's logo as a standalone SVG resource and included it in
  the M5 package file list. User-facing progress, navigation, errors, actions,
  accessibility labels, and metadata are localized in Portuguese.
- The application was launched in the available graphical session for a smoke
  check. That host is GNOME Shell 46/X11, not the target GNOME 50+/Wayland, and
  the full visual/interaction matrix has not been completed.
- The smoke launch emitted GTK theme parser warnings from the installed host
  theme and two `GtkImage` baseline warnings. The app remained open; their
  source and impact on the target GNOME session were not established.
- `./scripts/check-package.sh` and package-tree/resource validation passed
  after adding the logo. Building/installing a `.deb` is still unavailable on
  this host because `debhelper`/`dh` is not installed.
- Follow-up regression fix: Apply was disabled whenever the renderer had no
  D-Bus owner, preventing the explicit Apply action from reaching the
  renderer's activation/development-start path. It is now actionable until
  this particular item is confirmed active. Import success now validates and
  inserts the committed item into the live library model, uses the same
  centralized library-view refresh path, and falls back to a library reload if
  direct insertion encounters a filesystem inconsistency.
- Added deterministic coverage for Apply being available with a stopped
  renderer and for adding an imported item to the current in-memory library.
  The manual GNOME import/Apply flow still needs runtime confirmation.
- Host is Ubuntu 24.04.5, GTK 4.14.5, Libadwaita 1.5.0, GNOME Shell 46 on X11;
  it is not the supported GNOME 50+ Wayland target. M5 package build remains
  unvalidated on this host because debhelper is absent.

## Discoveries and decisions

- Battery preferences are renderer-owned while active and include
  `pause-on-battery` plus optional `pause-on-low-battery-only`; the app keeps
  them available and persistent while the renderer is stopped. Fullscreen and
  lock remain automatic policies with no preference API.
- Existing renderer status has no monitor inventory/geometry. Displays content
  must be limited to verified environment/state until a justified contract is
  available; adding a new backend contract is outside the UI-first scope.
- The desktop playback prototype is not yet a real background. Product copy
  must not repeat the HTML's inaccurate promise that playback is applied to
  all physical screens.

## Post-implementation notes

The implemented UI scope passes the repository checks. The execution plan
remains active because representative visual, accessibility, import/preview,
package-install, and GNOME 50+/Wayland runtime validation have not been
performed. Do not mark the milestone complete until those checks are run in a
suitable environment.
