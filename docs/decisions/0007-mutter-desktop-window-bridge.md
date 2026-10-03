# ADR-0007: Use a guarded Mutter desktop-window bridge

## Status

Accepted for an experimental implementation; GNOME 50+ Wayland runtime
validation is still required before this is considered supported.

## Context

Apply previously opened the renderer as an ordinary GTK player window. GTK4
does not provide a supported API to insert a paintable from a separate process
into GNOME Shell's background actor. Reparenting compositor-owned actors into
private Shell groups would make the Shell extension responsible for fragile
actor ownership and cleanup.

## Decision

Keep decoding and the GTK/GStreamer surface in the renderer process. Give its
top-level a fixed GTK application ID, and have the small Shell extension
discover it through Mutter's `window-created` signal and initial window list.
The extension sets its Mutter window type to `DESKTOP`, excludes it from window
lists, sticks it across workspaces, sizes it to the primary monitor, and lowers
it. The renderer sets a transparent input region and non-focusable,
undecorated window properties.

Before Apply, the renderer requires `SetDesktopIntegrationReady(true)` from the
extension. If readiness is absent or withdrawn, it refuses Apply or stops the
active output; it must never fall back to a normal player window. The extension
does not decode media, poll state, or reparent compositor actors.

## Consequences

- Playback remains in one external renderer; Shell work is limited to Mutter
  window lifecycle/placement and existing visibility events.
- The current slice targets one wallpaper output (the primary monitor); the
  extension recalculates its geometry after monitor topology changes.
- The Mutter window-type operation is version-sensitive and must be tested on
  GNOME 50+ Wayland, including stacking, Overview, workspaces, notifications,
  lock/unlock, and teardown.
- The available developer session is GNOME 46 on X11 and cannot validate the
  target behavior. The bridge is explicitly experimental until that test is
  performed.
- Disabling the extension stops playback to avoid leaving a stale desktop
  surface.

## Alternatives considered

- Leave the surface as an ordinary GTK window: rejected because it does not
  become a desktop background.
- Reparent/clone `Meta.WindowActor` into private `Main.layoutManager` groups:
  rejected because ownership, cleanup, and compatibility risks are higher.
- Change GNOME's static wallpaper setting or use X11-specific window hints:
  rejected because they do not satisfy the session/Wayland architecture.
