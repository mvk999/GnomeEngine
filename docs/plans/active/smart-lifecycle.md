# M3: Smart Lifecycle & Adaptive Performance

## Goal

Pause the active wallpaper whenever the desktop cannot meaningfully show it,
and resume only after every independent pause condition has cleared. Lifecycle
updates must be event-driven and renderer-owned; no polling, per-frame IPC, or
extra daemon.

## Current lifecycle architecture

There is no implemented lifecycle architecture yet. The only executable is a
Rust GTK/GStreamer preview that takes a local video path, creates one
`playbin`, and shows it in a normal GTK window. The GNOME Shell extension only
logs `enable()` and `disable()`. No renderer D-Bus connection or shared
surface/background bridge exists.

The renderer baseline was repaired in commit `5bb791a` after installing the
documented build dependencies. `./scripts/check.sh` now passes locally. The
check compiles the prototype but has no unit tests yet.

## Current renderer state model

No renderer state machine exists. There are no `Stopped`, `Loading`,
`Playing`, `Paused`, or `Error` states; no `ApplyVideo`, `Pause`, `Resume`,
`Stop`, or `GetStatus` D-Bus methods; and no state or playback-error signals.
The only current lifecycle is GTK activation and setting the pipeline to
`NULL` during application shutdown.

M2 remains a design document at
`docs/plans/active/renderer-dbus-service.md`, not a service implementation.
M1 remains incomplete as documented in
`docs/plans/active/gnome-background-integration.md`.

## Event sources researched

- The installed system is Ubuntu 24.04.5 with GNOME Shell 46.0. It is below
  the supported GNOME 50+ Wayland target, so no local GNOME 50 API or runtime
  behavior has been verified.
- The official Mutter documentation currently served during planning describes
  API 51. It lists `Meta.Display::in-fullscreen-changed`,
  `get_monitor_in_fullscreen()`, `Meta.MonitorManager::monitors-changed`, and
  `power-save-mode-changed`. These are research leads only; do not assume their
  signatures or availability in GNOME 50 until checked against the GNOME 50
  source/API and runtime.
- The official UPower reference exposes the read-only `OnBattery` property.
  The intended source is the system bus, with one initial property read and
  property-change subscription. UPower absence must leave rendering usable.
- The systemd-logind reference documents `PrepareForSleep(bool)` before and
  after sleep. The intended source is the system bus; no inhibitor lock is
  planned. Runtime availability and resume ordering remain untested.
- Lock detection is expected to use GNOME session-mode transitions, with an
  initial snapshot and event subscription. The exact GNOME 50 `Main.sessionMode`
  API, `unlock-dialog` lifetime, and extension-review implications still need
  verification against the target release.

Research references:

- [Mutter Meta.Display, currently API 51](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Display.html)
- [Mutter Meta.MonitorManager, currently API 51](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.MonitorManager.html)
- [UPower reference manual](https://upower.freedesktop.org/docs/UPower/)
- [systemd-logind D-Bus API](https://wiki.freedesktop.org/www/Software/systemd/logind/)

## Pause policy design

Use one set of independent reasons, owned by one source each:

| Reason | Owner | Initial policy |
| --- | --- | --- |
| `manual` | Renderer API | Set by `Pause()`, removed by `Resume()` |
| `fullscreen` | GNOME Shell extension | Pause only when every output rendered by the current M1 path is fullscreen |
| `screen-locked` | GNOME Shell extension | Pause while the user session is locked |
| `display-off` | GNOME Shell extension | Pause when all relevant outputs are off |
| `on-battery` | Renderer UPower observer | Enabled by default; absence of UPower is non-fatal |
| `system-sleep` | Renderer logind observer | Always pause during sleep preparation |

Effective playback is allowed only when a wallpaper exists and the reason set
is empty. Removing one reason must never clear another. `Stop()` removes the
active wallpaper and pipeline independently of pause reasons. A newly applied
wallpaper inherits any active reasons.

The exposed renderer state remains small (`Stopped`, `Loading`, `Playing`,
`Paused`, `Error`). Status also reports sorted pause reasons. Repeated reason
updates are no-ops; invalid reason names are rejected. `Pause()` and `Resume()`
operate only on `manual`.

## Fullscreen strategy

Use Mutter fullscreen-change events and query current monitor fullscreen state
on extension enable and after monitor changes. Do not scan windows on a timer.
Define a pure visibility decision against the outputs the actual M1 renderer
surface covers. If M1 renders only on the primary output, only that output
controls the reason; do not imply multi-monitor rendering support. If the
renderer covers multiple outputs, pause globally only when all covered outputs
are fullscreen.

The exact GNOME 50 signal signature and monitor enumeration API are unverified.
Resolve them against GNOME 50 source/runtime before implementing the adapter.

## Lock/unlock strategy

Use the supported session-mode change mechanism after confirming its GNOME 50
behavior. Connect event handlers before taking the initial snapshot, then
synchronize `screen-locked` immediately. Do not create UI, capture input, alter
the lock-screen background, or act in GDM. If `unlock-dialog` mode is required
to clear the reason correctly, document why and validate extension disable and
cleanup behavior in the target session.

## Display-power strategy

Use the GNOME/Mutter display power event and monitor-topology event after
confirming their exact GNOME 50 availability and enum values. Recompute
visibility, fullscreen coverage, and power state only after relevant events.
Do not hardcode undocumented enum integers. Remove `display-off` when at least
one output actually covered by the wallpaper is displaying the desktop.

## Battery strategy

The renderer reads UPower `OnBattery` once on the system bus and subscribes to
property changes. It owns the `on-battery` reason. Do not poll sysfs or let the
extension monitor battery too. If UPower cannot be reached, log one warning,
leave `on-battery` clear, and continue normal rendering.

## Suspend/resume strategy

The renderer observes logind `PrepareForSleep` on the system bus. `true` adds
`system-sleep`; `false` removes only that reason. Resume must preserve battery,
lock, display, fullscreen, and manual reasons. Do not acquire an inhibitor or
delay sleep. Validate whether the existing pipeline and M1 surface survive
resume before adding reconstruction behavior.

## Monitor-topology strategy

Subscribe to topology changes and recompute output identity, fullscreen
coverage, visibility, and display power from current state. Do not retain
monitor indexes as stable identities. The current repository has no established
multi-monitor surface mapping, so policy must follow the actual M1 coverage.

## D-Bus changes

After M2 exists, keep its `ApplyVideo`, `Pause`, `Resume`, `Stop`, and
`GetStatus` interface. Add one validated method such as
`SetPauseReason(reason: string, active: bool)` for automatic policy sources.
Expose `pauseReasons` in status and emit `PauseReasonsChanged` only on actual
set changes. Keep `StateChanged` for effective state transitions. D-Bus remains
control/status only; no frame or per-frame messages.

The service stays on the session bus. UPower and logind observers use the system
bus. The extension uses an asynchronous GIO proxy, watches service-name
ownership, tolerates service restart, and sends a current snapshot immediately
when the service appears.

## Testing strategy

Once M2 is present, add deterministic policy tests for empty reasons, manual
pause/resume interactions, duplicate additions, absent removals, simultaneous
conditions, apply while paused, and stop while paused. Add pure fullscreen
coverage tests for one and multiple rendered outputs. Test D-Bus reason
validation, status, signals, and manual API interaction without mocking the
whole system bus.

Manual GNOME checks are required for fullscreen, lock/unlock, display power,
monitor hotplug, extension reload, renderer restart, suspend/resume, and
visible-secondary-monitor behavior. Record unsupported hardware cases as not
validated. Never automate real suspend in CI.

## Performance strategy

Compare playing, each available paused condition, and stopped under one
documented media/session setup. Record GNOME/Mutter version, session type,
video codec/resolution/FPS, CPU, RSS, duration, and decoder where observable.
Do not claim a numeric target or hardware decoding without measurements.
Paused playback must set GStreamer to `PAUSED`, retain position, and stop
decoding/rendering frames as far as the stack permits. The extension remains
signal-driven and idle without a renderer.

## Risks

- M1 background placement and M2 renderer service are both absent, so M3 has no
  integration point yet.
- The local GNOME 46 session cannot validate the GNOME 50+ Wayland target.
- The online Mutter reference found during planning is API 51; GNOME 50-specific
  API verification remains open.
- Lock/session-mode lifetime, monitor power semantics, and resume ordering may
  expose version-specific behavior.
- The current M1 path has no real output mapping; multi-monitor policy must not
  claim broader support than the surface integration provides.
- UPower or logind can be unavailable in development, VM, or container setups.

## Implementation slices

The following slices are blocked until the repository has a working M1
background integration and M2 session-bus renderer service. This plan does not
reimplement either milestone.

1. **Unblock prerequisites** — complete/validate M1 and M2 independently; keep
   their plans and acceptance evidence accurate.
2. **Pure lifecycle policy** — add the reason set and deterministic tests in
   the renderer, preserving simple effective states.
3. **D-Bus lifecycle contract** — add reason update, status, signal, validation,
   and idempotence tests to the existing M2 service.
4. **Renderer pause control** — derive GStreamer state from active wallpaper
   plus reasons; test apply, stop, error, and pause/resume position behavior.
5. **System power observers** — add optional event-driven UPower and logind
   observers to the renderer with clean failure behavior.
6. **GNOME lifecycle controller** — after GNOME 50 API confirmation, implement
   event-driven fullscreen, lock, display power, and topology snapshots with
   asynchronous renderer proxy/name watching.
7. **Recovery and integration checks** — validate restart/resync, cleanup,
   multi-reason interaction, suspend/resume, and runtime performance; update
   architecture, testing, and measurement docs.

## Acceptance criteria

- [ ] M1 background bridge and M2 renderer service exist and are validated as
  the actual integration base.
- [ ] Baseline `./scripts/check.sh` passes before M3 implementation.
- [ ] Pause reasons are an independent set with explicit owners.
- [ ] Manual pause/resume changes only the manual reason.
- [ ] Effective playback derives from active wallpaper and all pause reasons.
- [ ] `GetStatus` exposes sorted reasons; invalid names fail; duplicate changes
  emit no duplicate signals.
- [ ] Applying while any automatic reason is active remains paused.
- [ ] Stop during automatic pause stays stopped after reasons clear.
- [ ] Fullscreen, lock, display power, and monitor changes are event-driven and
  use the verified GNOME 50 API.
- [ ] UPower and logind integrations are event-driven and fail non-fatally.
- [ ] Renderer appearance/restart receives a current Shell snapshot before
  automatic playback can waste frames.
- [ ] Multiple reasons compose correctly and removing one cannot resume early.
- [ ] GStreamer pauses and resumes the same playback position.
- [ ] `./scripts/check.sh` passes; deterministic lifecycle tests pass.
- [ ] Runtime and performance results are recorded without unsupported claims.
- [ ] No polling, timer-based window scans, per-frame IPC, extra daemon, or
  lock-screen UI is introduced.

## Progress

- [x] Inspected branches and confirmed the only feature branch was already
  merged into `main`; it contained the M1 skeleton and plans, not M2 code.
- [x] Installed Rust stable, `rustfmt`, Clippy, `pkg-config`, GTK4 and GStreamer
  development packages; canonical checks now pass.
- [x] Audited M2 APIs and implementation: no renderer service or D-Bus code is
  present in `renderer/` or `extension/`.
- [x] Recorded the M3 design and integration gates in this plan.
- [ ] Complete and validate M1 background integration.
- [ ] Implement and validate M2 renderer service.
- [ ] Verify GNOME 50 APIs and run lifecycle implementation slices.

## Discoveries

- The renderer initially failed with current Rust stable because its
  `seek_simple` call used an incompatible signature. Clippy also showed that
  the returned `BusWatchGuard` was dropped immediately, which removed the bus
  watch; both were corrected before the green baseline commit.
- The remote feature branch was already an ancestor of `main`. It was deleted
  from origin as requested; no M2 implementation was present there.
- The local desktop is GNOME Shell 46. The public Mutter documentation found
  during research is version 51, so those APIs are not accepted as GNOME 50
  evidence.
- The M2 D-Bus plan exists, but its implementation and its unit/runtime tests do
  not.
- The canonical check currently passes, but `cargo test` reports zero tests.

## Decisions

- Keep M3 blocked until M1 and M2 supply a real background and renderer control
  service; do not build duplicate foundations under this milestone.
- Use a set of independent pause reasons with one owner per reason.
- Renderer service stays on the session bus; UPower and logind observations use
  the system bus.
- Use events and snapshots, not recurring timers or polling.
- Treat GNOME 51 documentation only as a research lead; verify target APIs
  against GNOME 50 before implementation.

## Post-implementation notes

Pending. Keep this plan under `active/` until the acceptance criteria that
require target GNOME runtime behavior and performance evidence have been
completed. At present, M1 and M2 are prerequisite blockers.
