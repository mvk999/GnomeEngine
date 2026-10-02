# GNOME Integration Design

## Current state

There is no GNOME Shell extension or desktop wallpaper integration in the
repository. The renderer's `GdkPaintable` is shown by a GTK widget in a normal
application window. GTK4's paintable output alone does not attach that content
to Mutter's desktop background layer.

## Constraint

The Shell extension runs inside GNOME Shell and must remain minimal and
reversible. It may eventually identify renderer surfaces, track monitor
topology and visibility, and request pause/resume. Media decoding and heavy I/O
must stay in an external process.

The key unresolved design problem is how to present an external renderer's
content in the background layer without normal window behavior, focus, Alt+Tab,
Overview, or pointer interception. The project has not validated a stable
public GNOME API for this surface handoff. Private Shell actors or window-group
reparenting are not accepted as a design until version-specific behavior and
failure cleanup are tested in GNOME 50+ Wayland.

## Validation gate

Before implementing the bridge:

1. Research current official GNOME Shell, Mutter, GTK, and GStreamer APIs.
2. Prototype in a GNOME 50+ Wayland nested/development session.
3. Verify surface identity, monitor geometry/scale, input pass-through,
   Overview, workspaces, fullscreen, lock/unlock, reload, and crash cleanup.
4. Record the selected mechanism and rejected alternatives in an ADR.

If no safe and maintainable surface handoff can be demonstrated, revisit the
renderer output architecture instead of hiding a normal GTK window behind
timing or stacking hacks.
