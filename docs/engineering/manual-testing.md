# Manual Testing

The desktop surface bridge is experimental. A successful build or GTK preview
does not establish correct Mutter stacking, focus, or input pass-through.

For a graphical validation run, record GNOME Shell version, distribution,
session type, GPU/driver, test video codec/resolution/FPS, and exact steps.

## Renderer and desktop-surface smoke check

1. Check whether `gtk4paintablesink` is discoverable. If it is absent, confirm
   the platform's GTK media backend is installed; the renderer should select
   the GtkVideo fallback and report that choice in its log.
2. On GNOME 50+ Wayland with the GnomeEngine extension enabled, apply a local
   video through the app.
3. Confirm it appears behind normal windows as the desktop, with no player
   window, input interception, Alt+Tab entry, or Overview entry.
4. Confirm audio is discarded and playback loops; Stop restores the unchanged
   static GNOME background.
5. Try an invalid path and an unsupported codec; record the observed error.
6. Disable the extension and verify the renderer closes the desktop surface.
   With the extension absent, Apply must fail without presenting a normal GTK
   video window.

## M3 smart lifecycle checklist

Run only in a disposable or nested supported GNOME 50+ Wayland session.

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
- [ ] renderer itself never activates fullscreen pause on the primary output
- [ ] target stacking leaves panel, notifications, and system dialogs above wallpaper
- [ ] monitor topology change resizes the wallpaper surface to the primary output
- [ ] measure playing, paused, and stopped CPU/RSS with identical media/session

Record unavailable battery, display, suspend, multi-monitor, or performance
tests explicitly; never infer them from unit tests.

## M4 application checklist

Run `cargo build --workspace`, then `cargo run -p gnomeengine` in a graphical
session. Applying requires a supported GNOME session with the extension enabled;
otherwise the renderer should fail closed without a player window.

- [ ] A fresh XDG data directory opens to the Library empty state.
- [ ] Import one valid local video; title, static thumbnail, and available
  metadata appear without blocking the window.
- [ ] The original source remains unchanged; restart the app and confirm the
  managed copy and cached thumbnail persist.
- [ ] Import invalid media; a concise error appears and no partial card remains.
- [ ] Open detail; one muted preview starts. Back stops/releases it.
- [ ] Apply from detail with the extension enabled; verify the renderer surface
  becomes desktop content. Without the extension, verify a clear error and no
  visible GTK player window.
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

## M6 GUI V1 checklist

The redesigned GUI has been launched in the available GNOME Shell 46/X11
session for a smoke check. That is not the supported GNOME 50+ Wayland target
and does not complete visual acceptance.
That launch kept the window open but logged GTK CSS parser warnings from the
host theme and two `GtkImage` baseline warnings; their source and impact on the
target session are not established and should be investigated during the
target graphical review.

- [ ] Review Library with a fresh `XDG_DATA_HOME`; confirm logo, Portuguese
  empty copy, Import action, and no mock cards.
- [ ] Import valid and invalid videos; confirm the indeterminate progress
  window covers real work only, errors use friendly copy, and a successful
  import updates the visible library immediately without restarting the app
  and displays the actual cached thumbnail.
- [ ] Review real card thumbnails, titles, dimensions, duration, active state,
  and keyboard activation; verify cards do not create video pipelines.
- [ ] Open detail, check its muted live preview and available metadata. Apply
  while the renderer is stopped must trigger activation/start rather than be
  disabled; verify Apply and Stop against D-Bus state, then navigate away and
  confirm preview teardown.
- [ ] Visit Displays and Settings; confirm no monitor data is fabricated and
  only the existing battery-pause preference is configurable.
- [ ] Exercise renderer unavailable/recovery state and About.
- [ ] Review wide, medium/tiled, and narrow layouts, keyboard navigation,
  accessible labels, and the system light/dark schemes.
- [ ] Close the GUI with renderer playback active; verify the app exits without
  calling Stop, then reopen and compare renderer state.

Intentional differences from the HTML: mobile bottom navigation is replaced by
native adaptive split navigation; the Displays page reports only known session
and renderer information because no monitor inventory API exists; the app
uses the system font and color scheme; demo controls, mock wallpapers, and
unsupported settings are omitted. The renderer integration uses Mutter's
desktop window type and still requires the target-session validation below.

## M5 Ubuntu package acceptance

Run the package build on Ubuntu 26.04 amd64 with build tools installed. The
build script itself must not use sudo or modify system paths.

- [ ] `./scripts/check-package.sh` passes desktop, AppStream, extension, D-Bus
  activation, version, and SPDX checks.
- [ ] `./scripts/build-deb.sh` builds `dist/gnomeengine_<version>_amd64.deb`.
- [ ] `dpkg-deb -I` and `dpkg-deb -c` show the expected package metadata and
  only the app, renderer, desktop data, icon, service, and UUID-matched
  extension; no build tree or Node modules are present.
- [ ] Install explicitly with `sudo apt install ./dist/gnomeengine_*.deb` and
  verify `gnomeengine` appears in the app grid and `gnome-extensions info`
  recognizes the system extension.
- [ ] Verify the renderer starts through session D-Bus activation only after
  Apply, and that no package script enabled the extension or changed user
  preferences.
- [ ] Remove with `sudo apt remove gnomeengine`; system package files disappear
  while the user's library/configuration remain.
- [ ] Reinstall/upgrade and confirm managed wallpaper data remains intact.
- [ ] Run `appstreamcli validate`, `desktop-file-validate`, and `lintian` when
  installed; record unavailable tooling rather than implying it passed.

The current host is Ubuntu 24.04 GNOME 46/X11 and lacks `debhelper`, so it has
not built or installed the M5 package. These acceptance items remain pending
until the Ubuntu 26.04 package CI or a target installation verifies them.
