# Battery-aware settings and low-battery policy

## Goal

Make battery-related controls usable while the renderer service is absent, and
support an opt-in policy that pauses video only on battery at 20% charge or
below. Normal, non-fullscreen windows covering part of the desktop must not
pause wallpaper playback; actual fullscreen, lock, display-off, and suspend
reasons remain independent.

## Current behavior and evidence

- The Settings switch is disabled whenever the renderer D-Bus service is not
  running, so users cannot change the setting that controls whether playback
  pauses on battery.
- The renderer persists `pause-on-battery` in
  `$XDG_CONFIG_HOME/gnomeengine/lifecycle.ini` and samples only UPower
  `OnBattery`; it does not observe charge percentage.
- The GNOME extension adds `fullscreen` only for an actual fullscreen window
  on the primary output. A normal partially covering window is not a pause
  condition.
- UPower exposes aggregate `Percentage` on
  `/org/freedesktop/UPower/devices/DisplayDevice`; Noble host query succeeded.

## Chosen behavior

- Keep the existing master `pause-on-battery` preference, default enabled.
- Add `pause-on-low-battery-only`, default disabled. When enabled, battery
  pause applies only while on battery and reported percentage is `<= 20.0`.
- If percentage is unavailable, low-battery-only mode fails open and continues
  playback; all non-battery pause reasons retain their existing behavior.
- The GTK app can load/save these preferences while the renderer is absent.
  When the renderer is present, it remains the live policy authority and the
  same preference file is updated through D-Bus.
- Changing a preference must not activate an otherwise idle renderer.

## Implementation slices

1. [x] Add regression tests for threshold boundaries, missing percentage, AC
   power, and independent lifecycle reasons.
2. [x] Extend renderer config, UPower event/snapshot observation, status, and
   D-Bus setters for the low-battery-only option.
3. [x] Add app-side read/write fallback to the same config format so settings
   remain enabled and persistent with no running service.
4. [x] Rework Settings controls to remain usable offline. The threshold
   preference remains editable even when the master battery pause is off; in
   that state it is stored but has no effect until the master option is enabled.
5. [ ] Complete runtime validation, package checks, and manual GUI checks.

## Testing and acceptance

- `./scripts/check.sh` passes, including battery-policy regression tests.
- Setting either option with no renderer running updates the shared config and
  remains visible after app restart without spawning the renderer.
- With renderer running, settings update live and playback reconciles
  immediately.
- In threshold-only mode: 20% pauses; above 20% plays; AC always plays;
  unavailable percentage does not pause.
- In regular pause-on-battery mode, behavior remains unchanged.
- A partial, non-fullscreen window does not add `fullscreen`; a real fullscreen
  app still does. Manual graphical checks are recorded separately.

## Progress

- [x] Inspected current renderer policy, UPower monitor, app client, Settings
  UI, and existing lifecycle documentation.
- [x] Verified UPower `DisplayDevice.Percentage` is available on the current
  host (read-only system-bus query).
- [x] Implement tested low-battery policy and live percentage observation.
- [x] Enable and persist settings without renderer activation.
- [ ] Complete fast checks and manual validation.

Fast validation currently passes: `./scripts/check.sh` (25 app tests, 24
renderer tests, extension syntax, formatting, and Clippy). UPower's live
DisplayDevice percentage was confirmed via system D-Bus. The GUI controls and
physical battery threshold transitions still require manual validation.

## Discoveries

- The greyed-out switch is explicitly tied to `RendererStatus.available`,
  creating a chicken-and-egg UI: an inactive renderer prevents changing its
  own preference.
- The previous manual renderer status showed only `on-battery`, not
  `fullscreen`; partial-window behavior is already separated in policy.
