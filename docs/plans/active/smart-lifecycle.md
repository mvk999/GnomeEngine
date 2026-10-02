# M3: Smart Lifecycle & Adaptive Performance

## Goal

Pause the active wallpaper whenever the desktop cannot meaningfully show it,
and resume only after every independent pause condition has cleared. Lifecycle
updates must be event-driven and renderer-owned; no polling, per-frame IPC, or
extra daemon.

## Current lifecycle architecture

The renderer is a Rust GTK/GStreamer service owning a session-bus name. It
exports control/status methods and reconciles playback against independent
pause reasons. The Shell extension sends event-derived fullscreen, lock, and
display conditions. The renderer still presents a normal GTK preview window;
there is no shared surface/background bridge, so M1 remains incomplete.

## Current renderer state model

The renderer state model is `Stopped`, `Loading`, `Playing`, `Paused`, and
`Error`. `LifecyclePolicy` stores a validated sorted set of six pause reasons.
Status includes pause reasons and active-wallpaper state. `PauseReasonsChanged`
fires only when the set changes; state signals represent effective state.
GStreamer transitions to PAUSED/PLAYING when there is an active pipeline.

The minimum M2 session-bus service was implemented as a directly necessary
prerequisite for this milestone. Its D-Bus interface was smoke-tested. M1
background placement remains incomplete.

## Event sources researched

- The installed system is Ubuntu 24.04.5 with GNOME Shell 46.0 / Mutter 14 on
  X11. It is below the supported GNOME 50+ Wayland target.
- The installed Mutter 14 typelib confirms `Meta.Display::in-fullscreen-changed`,
  `get_monitor_in_fullscreen()`, `get_primary_monitor()`,
  `Meta.MonitorManager::monitors-changed`, and
  `power-save-mode-changed`. These shapes are locally introspected on GNOME 46;
  GNOME 50 runtime compatibility is still unverified.
- The official UPower reference exposes the read-only `OnBattery` property.
  The intended source is the system bus, with one initial property read and
  property-change subscription. UPower absence must leave rendering usable.
- The systemd-logind reference documents `PrepareForSleep(bool)` before and
  after sleep. The intended source is the system bus; no inhibitor lock is
  planned. Runtime availability and resume ordering remain untested.
- Lock detection uses `Main.sessionMode` updates and `currentMode` snapshots;
  the extension manifest includes `user` and `unlock-dialog`. Target release
  review and runtime behavior remain unverified.

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

The implementation queries the primary output on enable and after event-driven
topology changes. Since no actual background output mapping exists, this is a
provisional single-output policy, not evidence of multi-monitor background
support. GNOME 50 validation remains open.

## Lock/unlock strategy

Use `Main.sessionMode`'s `updated` event and `currentMode === 'unlock-dialog'`.
Handlers connect before the initial snapshot. The extension is declared for
`user` and `unlock-dialog`, with no UI, input capture, or GDM behavior. Disable
clears owned reasons except that it retains `screen-locked` if disabled while
locked, avoiding playback under lock at the cost of a documented stale-reason
edge case until renderer restart or a later sync.

## Display-power strategy

Use `power-save-mode-changed` as an event trigger and the public
`get_is_builtin_display_on()` state for systems with a built-in panel. Mutter
does not expose an aggregate off-state for arbitrary external outputs through
this adapter; such systems fail open. Topology changes recompute state. No
enum integers are hardcoded. GNOME 50 and hardware runtime checks remain open.

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

Keep `ApplyVideo`, `Pause`, `Resume`, `Stop`, and `GetStatus`. Add validated
`SetPauseReason(reason: string, active: bool)`. Status exposes sorted
`pauseReasons` and `wallpaperActive`; `PauseReasonsChanged` emits only after a
set change. `StateChanged` reports effective state transitions. D-Bus remains
control/status only.

The service stays on the session bus. UPower and logind observers use the system
bus. The extension uses asynchronous GIO D-Bus calls, watches service-name
ownership, tolerates service restart, and sends the current reason snapshot
immediately when the service appears.

## Testing strategy

Deterministic tests cover empty/manual/duplicate/unknown reasons, fullscreen
coverage, manual-vs-fullscreen Resume, pause reason status, and Stop while an
automatic reason is active. A live D-Bus smoke test covered introspection,
GetStatus, SetPauseReason, and invalid-reason rejection. Applying media while
paused and actual signal de-duplication need a usable GTK media sink/session.

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

- M1 background placement remains absent; lifecycle currently controls a GTK
  preview pipeline, not a real desktop wallpaper.
- GNOME Shell 46/X11 cannot validate the GNOME 50+ Wayland target.
- The online Mutter reference found during planning is API 51; GNOME 50-specific
  API verification remains open.
- Lock/session-mode lifetime, monitor power semantics, and resume ordering may
  expose version-specific behavior.
- The current M1 path has no real output mapping; primary-monitor behavior is
  provisional and multi-monitor wallpaper visibility is unvalidated.
- Mutter's public power API reports built-in-panel state only; external-only
  display-off policy is unavailable and fails open.
- UPower or logind can be unavailable in development, VM, or container setups.

## Implementation slices

1. **Renderer service foundation** — implement the directly necessary M2
   session-bus service and single-name ownership.
2. **Pure lifecycle policy** — implement validated reasons and deterministic
   tests.
3. **D-Bus lifecycle contract** — add reason update, status, signal, validation,
   and duplicate no-op behavior.
4. **Renderer pause control** — derive GStreamer state from active wallpaper
   plus reasons.
5. **System power observers** — subscribe to UPower and logind on the system
   bus with non-fatal failure behavior.
6. **GNOME lifecycle controller** — connect Mutter/session events, async D-Bus
   calls, renderer name watching, and initial/restart synchronization.
7. **Docs and runtime gates** — record implementation, manual tests, and
   performance measurements where the environment permits.

## Acceptance criteria

- [x] M2 renderer service and lifecycle API exist; D-Bus smoke test passes.
- [ ] M1 background bridge exists and validates actual desktop visibility.
- [x] Baseline `./scripts/check.sh` passed before M3 implementation.
- [x] Pause reasons are an independent set with explicit owners.
- [x] Manual pause/resume changes only the manual reason.
- [x] Effective playback derives from active wallpaper and all pause reasons.
- [x] `GetStatus` exposes sorted reasons; invalid names fail; duplicate changes
  emit no duplicate signals.
- [x] Applying while any automatic reason is active remains paused by
  controller reconciliation (media runtime unavailable for an end-to-end test).
- [x] Stop during automatic pause stays stopped after reasons clear.
- [x] Fullscreen, lock, display-power trigger, and monitor topology updates are
  event-driven using Mutter API introspected on GNOME 46.
- [ ] GNOME 50 Wayland API/runtime behavior is validated.
- [x] UPower and logind integrations are event-driven and fail non-fatally.
- [x] Renderer appearance/restart receives a current Shell snapshot before
  automatic playback can waste frames.
- [x] Multiple reasons compose correctly and removing one cannot resume early.
- [x] GStreamer is instructed to pause and resume the same pipeline (position
  retention has not been visually/runtime-validated).
- [x] `./scripts/check.sh` passes; deterministic lifecycle tests pass.
- [x] The only available performance sample is recorded with its stopped-only
  scope; playing/paused comparison remains unmeasured.
- [ ] No polling, timer-based window scans, per-frame IPC, extra daemon, or
  lock-screen UI is introduced.

## Progress

- [x] Inspected repository history, renderer, extension, and active plans.
- [x] Baseline `./scripts/check.sh` passed before implementation.
- [x] Implemented renderer service, lifecycle reasons, D-Bus state/status, and
  GStreamer state reconciliation.
- [x] Implemented event-driven UPower/logind observers and Shell fullscreen,
  lock, topology, and built-in-panel power handling.
- [x] Added 15 policy/controller tests, D-Bus smoke tests, ADR, design,
  architecture, manual testing, and a stopped-state performance sample.
- [ ] Validate real desktop visibility and lifecycle on GNOME 50+ Wayland.
- [ ] Test hardware power transitions, suspend/resume, monitor hotplug, and
  playing/paused performance.

## Discoveries

- The renderer initially failed with current Rust stable because its
  `seek_simple` call used an incompatible signature. Clippy also showed that
  the returned `BusWatchGuard` was dropped immediately, which removed the bus
  watch; both were corrected before the green baseline commit.
- The remote feature branch was already an ancestor of `main`. It was deleted
  from origin as requested; no M2 implementation was present there.
- The local Mutter 14 typelib reports fullscreen, topology, and power-save
  signals. This verifies the local API only, not GNOME 50.
- A live D-Bus call exposed an incorrect `GetStatus` reply signature; it was
  fixed, then GetStatus, SetPauseReason, and invalid-reason rejection passed.
- The `gtk4paintablesink` plugin is missing here, so media playback and
  paused-vs-playing performance could not be measured.

## Decisions

- Implement the minimum M2 renderer control service because M3 requires it, but
  do not claim it completes the missing M1 background integration.
- Use a set of independent pause reasons with one owner per reason.
- Renderer service stays on the session bus; UPower and logind observations use
  the system bus.
- Use events and snapshots, not recurring timers or polling.
- Treat local GNOME 46 introspection as evidence for this host only; target
  GNOME 50 validation remains mandatory.

## Post-implementation notes

Implementation and automated checks are complete for the available environment.
Keep this plan in `active/` until GNOME 50 Wayland, M1 background placement,
real media playback, physical power transitions, suspend/resume, hotplug, and
performance measurements have been validated.
