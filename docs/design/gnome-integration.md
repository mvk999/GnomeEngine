# GNOME Integration Design

## Current state

The GNOME Shell extension reports fullscreen, session lock, monitor topology,
and display visibility over D-Bus. It discovers the renderer's GTK surface by
its stable identity and uses a narrow version/backend bridge to place it in
the desktop layer. The renderer owns GStreamer and applies an empty GDK input
region so pointer events pass through the wallpaper surface.

## Constraint

This remains an experimental, version-specific bridge, not yet accepted as a
fully validated M1 implementation. GNOME 49+/Wayland keeps the modern
`Meta.Window.set_type(Meta.WindowType.DESKTOP)` route. GNOME 46/Wayland uses a
Shell-owned `Meta.WaylandClient` to spawn the same external renderer and calls
`make_desktop()` / `hide_from_window_list()` on its owned surface. GNOME
46/X11 uses the GDK X11 surface and pre-map EWMH desktop, sticky,
skip-taskbar/skip-pager, and all-workspaces hints; it reapplies the no-keyboard-
focus hint after GTK maps the window. Mutter/the X11 window manager retain
surface ownership; no actor reparenting, stacking polling, or window-management
CLI is used. Fullscreen policy excludes the renderer using bridge-appropriate
identity.

The D-Bus integration-ready handshake is fail-closed: the renderer refuses
Apply until the extension has advertised support, and stops/closes the surface
if the extension withdraws readiness. This prevents a normal GTK video player
window from being the fallback. The renderer also sets a stable GTK application
ID, disables decorations/focus, and rejects backends that cannot set an empty
input region. Decoding remains outside GNOME Shell.

## Validation gate

Before accepting any bridge as supported:

1. Inspect APIs for GNOME 46 through 50 and document version seams.
2. Test GNOME 46 Wayland and GNOME 50 Wayland in suitable separate sessions;
   test GNOME 46 X11 in a real `XDG_SESSION_TYPE=x11` session.
3. Verify surface identity, monitor geometry/scale, input pass-through,
   Overview, workspaces, fullscreen, lock/unlock, reload, and crash cleanup.
4. Record the selected mechanism and rejected alternatives in an ADR.

The implementation is guarded by runtime capability checks and remains
unavailable when the selected bridge cannot attach the surface. The local
validation session is GNOME 46 on X11, which is one of the M7 targets; however,
the extension is not installed in the current session, so the renderer-only
Apply/Stop and EWMH smoke evidence does not establish Shell integration. GNOME
46 Wayland desktop semantics and GNOME 50 Wayland remain separate acceptance
gates. No target is considered supported until the manual matrix is complete.

## Runtime contract

- `SetDesktopIntegrationReady(true)` is sent by the enabled Shell extension
  after installing event handlers; false stops active playback.
- The renderer uses one GTK top-level output on the primary monitor. Independent
  per-monitor content is not implemented.
- Wallpaper surfaces always target the full monitor rectangle, never a
  workspace work area. Work area is diagnostic only; panels and docks overlay
  the wallpaper and the renderer reserves no desktop space.
- On X11, reapply the full monitor rectangle after Mutter recognizes the EWMH
  desktop window, because the window manager can initially size it to the work
  area. A single geometry log reports monitor, work area, and renderer frame;
  monitor topology changes trigger an event-driven full-geometry update.
- The extension handles renderer windows using Mutter signals and monitor
  change events only; it has no timer or media work.
- Extension disable stops playback, so no stale desktop-class surface remains.
- Failure to classify the surface fails closed rather than presenting a normal
  player window.
