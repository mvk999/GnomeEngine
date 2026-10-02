# Smart lifecycle

## Ownership and flow

```text
GNOME Shell extension
  ├─ Mutter fullscreen + monitor topology ─┐
  ├─ GNOME session-mode lock state ────────┼─ session D-Bus ─▶ renderer
  └─ built-in display power (when exposed) ┘                  │
UPower OnBattery ──────────────── system D-Bus ───────────────┤
logind PrepareForSleep ────────── system D-Bus ───────────────┘
                                                               ▼
                                                     LifecyclePolicy
                                                               ▼
                                                    GStreamer PAUSED/PLAYING
```

The renderer is the sole playback authority. The extension reports desktop
conditions and never touches GStreamer. The renderer service uses the session
bus; system power observers use the system bus. None of these paths carries
video frames.

## Reasons and effective state

`LifecyclePolicy` stores a sorted set of independent reasons:

| Reason | Owner | Meaning |
| --- | --- | --- |
| `manual` | Renderer API | Explicit user/client pause |
| `fullscreen` | Shell extension | Fullscreen covers the output currently treated as the wallpaper output |
| `screen-locked` | Shell extension | Session mode is `unlock-dialog` |
| `display-off` | Shell extension | Built-in laptop panel is reported off by Mutter |
| `on-battery` | Renderer / UPower | UPower reports `OnBattery=true`; default policy pauses |
| `system-sleep` | Renderer / logind | `PrepareForSleep(true)` is active |

`Pause()` adds `manual`; `Resume()` removes only `manual`. Playback is
permitted only if a wallpaper is active and the set is empty. `Stop()` removes
the pipeline and active wallpaper, independently of automatic reasons. New
wallpaper playback reconciles against the current set, so it does not bypass
an active condition. GStreamer moves to `PAUSED` and resumes the existing
pipeline position when the final reason clears. Battery pausing defaults on in
an internal `LifecyclePolicyConfig`; there is no settings UI or persisted
configuration in this milestone.

The service exposes `SetPauseReason(reason, active)` for controlled lifecycle
updates, rejects names outside the six-value vocabulary, and emits
`PauseReasonsChanged(as)` only after an actual set change. `GetStatus()` returns
state, active video, last error, `pauseReasons`, and whether a wallpaper is
active. `StateChanged` remains reserved for effective renderer-state changes.

## Event sources and failure behavior

- Fullscreen: Mutter's `in-fullscreen-changed`; current state is queried on
  extension enable, topology changes, session-mode changes, and renderer name
  appearance. No window scan or timer is used.
- Lock: `Main.sessionMode` updates; `unlock-dialog` means locked. The extension
  is declared for `user` and `unlock-dialog` so it can keep the pause reason
  accurate through unlock. No lock-screen UI or input hooks are installed.
- Display power: Mutter's `power-save-mode-changed` triggers reconciliation.
  The public state used here is `get_is_builtin_display_on()`. Mutter does not
  expose a public aggregate “all outputs off” state through this adapter, so
  external-only systems fail open and are not covered by `display-off`.
- Topology: `monitors-changed` re-queries the primary monitor and fullscreen
  state; no monitor index is retained across changes.
- Battery: the renderer subscribes to UPower `PropertiesChanged` before
  requesting the initial `OnBattery` property. A signal observed during the
  initial read takes precedence over the snapshot. UPower/bus failure logs a
  warning and does not stop the renderer.
- Suspend: the renderer subscribes to logind `PrepareForSleep`. `true` adds
  `system-sleep`; `false` removes only that reason. No inhibitor is acquired.

The extension watches the renderer's well-known session-bus name. It keeps a
local event-derived snapshot while the renderer is absent and sends current
reason values on appearance/restart. Calls are asynchronous; errors are
warnings and do not escape into GNOME Shell callbacks. On normal extension
disable, extension-owned reasons are cleared, except `screen-locked` remains
active if disable occurs during `unlock-dialog` to avoid resuming under lock.
If that uncommon forced-disable case occurs, the reason may remain until the
renderer is restarted or a later extension sync; disabling while locked cannot
both retain safe lock behavior and guarantee a future unlock callback.

## Scope and validation limits

The current renderer still uses a normal GTK preview window and has no
established background-surface/output mapping. The extension therefore uses
the primary monitor as a conservative single-output policy, not as proof of
real wallpaper visibility or multi-monitor background support. The Mutter
signals and methods were introspected on the available GNOME Shell 46 / Mutter
14 host; the extension manifest targets GNOME 50. GNOME 50 runtime behavior
must be validated on a supported Wayland session before this design is
considered fully accepted.

System-bus power signals, renderer D-Bus methods, and policy interactions have
automated or smoke coverage. Physical battery changes, lock/unlock, suspend,
display power, monitor hotplug, and performance have not been runtime-tested
in this environment.
