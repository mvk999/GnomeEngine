# ADR 0005: Independent renderer pause reasons

- Status: Accepted
- Date: 2026-10-02

## Context

Fullscreen, lock, display power, battery policy, suspend, and manual pause can
be active simultaneously. A single paused boolean or an unconditional
`Resume()` would allow one event source to cancel another source's condition.

## Decision

The renderer owns a set of validated pause reasons. Manual `Pause()` and
`Resume()` modify only `manual`; each automatic source owns only its own reason.
The active wallpaper and the reason set jointly determine effective playback.
`Stop()` remains a separate operation that destroys the active pipeline.

## Consequences

- Clearing one condition cannot resume while another remains.
- Status and a dedicated D-Bus signal expose reason changes even when the
  effective state remains `Paused`.
- The renderer performs slightly more bookkeeping and validates the finite
  reason vocabulary at its D-Bus boundary.
- Automatic owners must reconcile initial state and clean up their own
  conditions when they disconnect.
