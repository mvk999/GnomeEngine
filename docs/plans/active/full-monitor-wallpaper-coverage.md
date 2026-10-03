# Full-Monitor Wallpaper Coverage

## Goal

Keep the wallpaper surface sized to each selected monitor's full geometry so
GNOME panels and docks remain layered above it. The workspace work area is
diagnostic only and must never size wallpaper content.

## Current discovery

On the available Ubuntu 24.04 / GNOME Shell 46 / X11 session, before the code
change, the primary monitor/root geometry was `1920x1080@0,0`, `_NET_WORKAREA`
was `1920x996@0,0`, and the mapped renderer window was `1920x996@0,0`. The
renderer had `_NET_WM_WINDOW_TYPE_DESKTOP`, sticky/skip-taskbar/skip-pager and
all-workspaces hints, with no `_NET_WM_STRUT` or `_NET_WM_STRUT_PARTIAL`.
This confirms an 84-pixel work-area-sized surface rather than a missing EWMH
desktop type or a strut reservation.

## Implementation

- X11 bridge reapplies Mutter's full monitor rectangle after recognizing the
  renderer's EWMH desktop window.
- GNOME 49+/Wayland retains its existing full-monitor `move_resize_frame`
  behavior; GNOME 46/Wayland reports geometry after `make_desktop()` without
  changing its ownership/size path.
- Geometry diagnostics report monitor, work area, and renderer frame in one
  Shell log line. Work area is read for observation only.
- X11 EWMH desktop/taskbar/workspace hints and no-input behavior are unchanged;
  no struts, polling, extra surfaces, or media pipelines are added.

## Validation

- [x] Confirm pre-change geometry mismatch and absence of X11 struts.
- [x] Add event-driven full-monitor resize for the X11 bridge and geometry log.
- [x] Document the full-monitor invariant and manual acceptance check.
- [x] `./scripts/check.sh`, extension syntax, and `git diff --check` pass.
- [ ] Reload the updated extension on GNOME 46/X11 and verify logged renderer
  geometry is `1920x1080@0,0`; visually confirm the bottom panel overlays video.
- [ ] Runtime-check GNOME 46/Wayland and GNOME 50/Wayland when available.
- [ ] Check top/side panel placement and multi-monitor topology where available.

## Performance and safety

Only window geometry is updated on renderer recognition and Mutter monitor
topology events. No polling, frame copying, additional decode work, panel
customization, or user preference changes are introduced.
