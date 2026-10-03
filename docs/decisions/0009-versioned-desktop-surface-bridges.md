# ADR-0009: Small Versioned Desktop-Surface Bridges

Status: Accepted

## Context

The original desktop integration assumed one Mutter API could classify an
externally D-Bus-activated GTK renderer surface on every target. GNOME 49
introduced the `Meta.Window` classification methods used by the modern bridge;
Mutter 14/GNOME 46 does not expose them. On GNOME 46 Wayland, `Meta.WaylandClient`
can classify only a client it owns, while the renderer is otherwise activated
by the session bus. On GNOME 46 X11, GTK4 has no window-type setter, but its
public X11 surface can be identified and configured with standard EWMH
properties before map.

## Decision

Keep the renderer core, GStreamer pipeline, renderer D-Bus control API,
lifecycle, library, and GUI shared. Allow only the desktop-surface integration
mechanism to diverge:

- GNOME 49+/Wayland retains `Meta.Window.set_type(DESKTOP)` and related APIs.
- GNOME 46/Wayland uses a minimal Shell integration D-Bus coordinator to start
  the same external renderer with `Meta.WaylandClient.spawnv()`, then uses
  ownership checks, `make_desktop()`, and `hide_from_window_list()`.
- GNOME 46/X11 uses GDK X11 plus EWMH desktop, sticky, skip-taskbar,
  skip-pager, and all-workspaces properties on the renderer surface before it
  is mapped.

Mutter 14's native `meta_wayland_client_new_indirect()` and
`meta_wayland_client_setup_fd()` symbols are not exposed in its GJS typelib.
They are not called through private FFI or guessed bindings. The existing
renderer D-Bus methods remain unchanged. No X11 window-management CLI,
polling, frame-copy fallback, or extra daemon is introduced.

## Consequences

- Each bridge must be runtime-tested for task switching, overview, workspaces,
  input transparency, lock/unlock, Stop, and lifecycle behavior before its
  Shell version is advertised.
- GNOME 46 Wayland's renderer process lifetime is coupled to the extension's
  owned Wayland client. Disabling that extension exits that exact child, while
  closing the GUI does not.
- EWMH behavior depends on Mutter's X11 implementation and must be validated
  in an actual `XDG_SESSION_TYPE=x11` session, not XWayland under Wayland.
- No support-matrix or extension metadata claim is made until the required
  runtime acceptance tests pass.
