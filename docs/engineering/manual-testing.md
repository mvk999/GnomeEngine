# Manual Testing

The current PoC only opens an ordinary GTK window. Manual playback checks do
not imply that desktop-background integration exists.

For a graphical validation run, record GNOME Shell version, distribution,
session type, GPU/driver, test video codec/resolution/FPS, and exact steps.

## Current prototype smoke check

1. Check whether `gtk4paintablesink` is discoverable. If it is absent, confirm
   the platform's GTK media backend is installed; the renderer should select
   the GtkVideo fallback and report that choice in its log.
2. Run the renderer with a local video containing a video stream.
3. Confirm the GTK window displays video, audio is discarded, and playback
   seeks to the beginning after end-of-stream.
4. Close the window and confirm the process exits without leaving playback
   running.
5. Try an invalid path and a video with an unsupported codec; record the
   observed error.

## M3 smart lifecycle checklist

Run only in a disposable or nested supported GNOME 50+ Wayland session. The
current GTK preview is not a desktop background, so successful lifecycle
signals do not establish desktop wallpaper visibility.

- [ ] renderer service starts and exports its D-Bus API
- [ ] video playback starts and manual Pause/Resume preserves position
- [ ] fullscreen automatically pauses and leaving fullscreen resumes
- [ ] a visible secondary rendered output prevents global fullscreen pause
- [ ] lock pauses; repeated lock/unlock does not stick or duplicate reasons
- [ ] battery transition pauses; AC reconnect resumes when appropriate
- [ ] display power changes pause/resume on supported built-in panel hardware
- [ ] suspend pauses; resume removes only `system-sleep`
- [ ] monitor hotplug recomputes fullscreen/output state without a crash
- [ ] duplicate reason updates do not emit duplicate reason-change signals
- [ ] removing one of multiple reasons never resumes early
- [ ] Stop while auto-paused remains Stopped after the condition clears
- [ ] applying while an automatic reason is active remains paused
- [ ] renderer restart receives a current Shell lifecycle snapshot
- [ ] renderer disappearance/crash does not crash GNOME Shell
- [ ] disabling/re-enabling the extension leaves no unintended stale reason
- [ ] Alt+Tab, Overview, workspace switching, and input behavior are unchanged
- [ ] measure playing, paused, and stopped CPU/RSS with identical media/session

Record unavailable battery, display, suspend, multi-monitor, or performance
tests explicitly; never infer them from unit tests.

## M4 application checklist

Run `cargo build --workspace`, then `cargo run -p gnomeengine` in a graphical
session. The app is native GTK4/Libadwaita, but its Apply action currently
controls the renderer's ordinary GTK preview window, not the GNOME background.

- [ ] A fresh XDG data directory opens to the Library empty state.
- [ ] Import one valid local video; title, static thumbnail, and available
  metadata appear without blocking the window.
- [ ] The original source remains unchanged; restart the app and confirm the
  managed copy and cached thumbnail persist.
- [ ] Import invalid media; a concise error appears and no partial card remains.
- [ ] Open detail; one muted preview starts. Back stops/releases it.
- [ ] Apply from detail; if the renderer is absent the app starts its sibling
  executable directly, waits for D-Bus, and applies the managed copy.
- [ ] Current renderer status and pause reasons are reflected; no status timer
  is active.
- [ ] Stop is explicit. Quit/close the app and verify it does not call Stop;
  reopen and compare the D-Bus status/current item.
- [ ] Change Pause on battery; verify the renderer stores the setting under
  XDG config and exposes the new policy via `GetStatus` after restarting the
  renderer.
- [ ] Remove an inactive item; confirm the managed directory is removed and
  the original remains. Remove an active item; Stop must succeed before the
  managed copy is deleted.
- [ ] Resize narrow and wide; check keyboard focus/actions, accessible names,
  and GNOME light/dark preference.
- [ ] Open/leave preview repeatedly and verify there is no accumulating
  renderer/preview process or persistent app process after quitting.

The host used for implementation is GNOME Shell 46 on X11; Gtk4's
`gtk4paintablesink` plugin is unavailable, although GTK's GStreamer media
backend is installed. The fallback compiles but still needs graphical runtime
validation. Mark graphical/runtime items above not validated on this host
rather than treating compilation or unit tests as visual validation.
