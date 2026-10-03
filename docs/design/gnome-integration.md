# GNOME Integration Design

## Current state

The GNOME Shell extension reports fullscreen, session lock, monitor topology,
and display visibility over D-Bus. It also discovers the renderer's GTK
surface by its fixed application ID and asks Mutter to treat that toplevel as
a `DESKTOP` window. The renderer owns GStreamer and applies an empty GDK input
region so pointer events pass through the wallpaper surface.

## Constraint

This is an experimental, version-specific bridge, not yet accepted as a
validated M1 implementation. The extension uses Mutter's `Meta.Window.set_type`
with `Meta.WindowType.DESKTOP`, hides the window from window lists, sticks it to
workspaces, sizes it to the primary monitor, and lowers it. Mutter retains
ownership of the compositor actor; the extension does not reparent actors into
private Shell groups. Fullscreen policy explicitly excludes the renderer's
window by application ID.

The D-Bus integration-ready handshake is fail-closed: the renderer refuses
Apply until the extension has advertised support, and stops/closes the surface
if the extension withdraws readiness. This prevents a normal GTK video player
window from being the fallback. The renderer also sets a stable GTK application
ID, disables decorations/focus, and rejects backends that cannot set an empty
input region. Decoding remains outside GNOME Shell.

## Validation gate

Before accepting this bridge as supported:

1. Inspect exact GNOME 50 Shell/Mutter behavior and document API assumptions.
2. Test in a GNOME 50+ Wayland nested/development session.
3. Verify surface identity, monitor geometry/scale, input pass-through,
   Overview, workspaces, fullscreen, lock/unlock, reload, and crash cleanup.
4. Record the selected mechanism and rejected alternatives in an ADR.

The implementation is guarded by runtime capability checks and remains
unsupported when the Shell cannot classify the surface as `DESKTOP`. The local
validation host is GNOME 46 on X11; it is outside the declared target and cannot
prove GNOME 50+ Wayland behavior. No runtime success is claimed until the
manual checklist is completed.

## Runtime contract

- `SetDesktopIntegrationReady(true)` is sent by the enabled Shell extension
  after installing event handlers; false stops active playback.
- The renderer uses one GTK top-level output on the primary monitor. Independent
  per-monitor content is not implemented.
- The extension handles renderer windows using Mutter signals and monitor
  change events only; it has no timer or media work.
- Extension disable stops playback, so no stale desktop-class surface remains.
- Failure to classify the surface fails closed rather than presenting a normal
  player window.
