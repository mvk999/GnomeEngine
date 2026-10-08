# Manual Testing

The desktop surface bridge is experimental. A successful build or GTK preview
does not establish correct Mutter stacking, focus, or input pass-through.

For a graphical validation run, record GNOME Shell version, distribution,
session type, GPU/driver, test video codec/resolution/FPS, and exact steps.

## M7 target identification

Run the full acceptance sequence separately on each exact target below. Capture
these values at the start of each run; XWayland applications do not count as
an X11 session.

| Ubuntu | GNOME Shell | Required `XDG_SESSION_TYPE` |
| --- | ---: | --- |
| 24.04 LTS | 46 | `wayland` |
| 24.04 LTS | 46 | `x11` |
| 26.04 LTS | 50 | `wayland` |

```sh
. /etc/os-release
printf 'OS: %s %s (%s)\n' "$PRETTY_NAME" "$VERSION_ID" "$VERSION_CODENAME"
gnome-shell --version
printf 'session=%s\n' "$XDG_SESSION_TYPE"
dpkg-query -W -f='${Package} ${Version}\n' \
  libgtk-4-1 libadwaita-1-0 gstreamer1.0-tools
```

For each target, install/enable the extension using the supported test workflow,
launch the installed or development GUI, import a legal local H.264 MP4, open
its detail page, confirm preview, and Apply. Verify desktop placement, close
the GUI while playback stays active, reopen it and confirm renderer state,
then Stop. Repeat Apply → Stop → Apply at least three times. Record the selected
sink, codec, resolution, FPS, decoder and graphics path when observable; mark
DMA-BUF only when runtime evidence confirms it.

Before the production metadata includes Shell 46, install a temporary test copy
whose metadata advertises only the Shell version under test. This leaves the
repository metadata untouched and does not disable GNOME's version validation
globally. Run from the repository root in the target test session:

```sh
test_root="$(mktemp -d)"
test_extension="$test_root/gnomeengine@mvk999.github.io"
mkdir -p "$test_extension"
cp extension/extension.js extension/metadata.json "$test_extension/"
shell_version="$(gnome-shell --version | awk '{print $3}')"
shell_major="${shell_version%%.*}"
case "$shell_major" in
  46)
    node --input-type=module - "$test_extension/metadata.json" <<'NODE'
import fs from 'node:fs';
const path = process.argv[2];
const metadata = JSON.parse(fs.readFileSync(path, 'utf8'));
metadata['shell-version'] = ['46'];
fs.writeFileSync(path, `${JSON.stringify(metadata, null, 2)}\n`);
NODE
    ;;
  50) ;;
  *) echo "Unsupported M7 test Shell: $shell_major" >&2; exit 2 ;;
esac
gnome-extensions pack "$test_extension" --out-dir "$test_root"
gnome-extensions install --force \
  "$test_root/gnomeengine@mvk999.github.io.shell-extension.zip"
gnome-extensions enable gnomeengine@mvk999.github.io
```

After the run, disable and remove the temporary user extension with
`gnome-extensions disable gnomeengine@mvk999.github.io` and
`gnome-extensions uninstall gnomeengine@mvk999.github.io`, then remove
`"$test_root"`. For packaged acceptance after metadata is widened, test the
system-installed extension from the `.deb` instead of this temporary copy.

## Renderer and desktop-surface smoke check

1. The renderer privately registers the bundled upstream `gtk4paintablesink`
   if the system factory is absent. Confirm renderer logs identify the selected
   sink; if static registration fails, verify the platform GTK media backend
   exists for the `GtkVideo` fallback. Do not infer the active EGL/GLX or
   DMA-BUF path from plugin compilation alone.
2. Repeat the application flow on all three M7 targets in the table above.
3. Confirm it appears behind normal windows as the desktop, with no player
   window, input interception, Alt+Tab entry, or Overview entry.
4. Confirm audio is discarded and playback loops; Stop restores the unchanged
   static GNOME background.
5. Try an invalid path and an unsupported codec; record the observed error.
6. Disable the extension and verify the renderer closes the desktop surface.
   With the extension absent, Apply must fail without presenting a normal GTK
   video window.

## M3 smart lifecycle checklist

Run the lifecycle checks on each M7 target. Use a disposable or nested session
for disruptive cases where possible. Record hardware-dependent cases as
unavailable when the required battery, display, monitor, lock, or suspend
facility is absent.

- [ ] renderer service starts and exports its D-Bus API
- [ ] video playback starts and manual Pause/Resume preserves position
- [ ] fullscreen automatically pauses and leaving fullscreen resumes
- [ ] an ordinary, non-fullscreen window covering part of the desktop does not
  pause playback; only a real fullscreen window on the primary display adds
  the `fullscreen` pause reason
- [ ] a visible secondary rendered output prevents global fullscreen pause
- [ ] lock pauses; repeated lock/unlock does not stick or duplicate reasons
- [ ] battery transition pauses; AC reconnect resumes when appropriate
- [ ] with "Somente com 20% de bateria ou menos" enabled, charge above 20%
  keeps playback running on battery; at 20% or below it pauses, and above the
  threshold it resumes. AC always clears the battery reason.
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
- [ ] wallpaper geometry log reports the full monitor rectangle, regardless of
  a smaller work area; on X11, `xprop` shows `_NET_WM_WINDOW_TYPE_DESKTOP` and
  no `_NET_WM_STRUT` or `_NET_WM_STRUT_PARTIAL`
- [ ] measure playing, paused, and stopped CPU/RSS with identical media/session

For the full-monitor coverage regression, compare monitor, workspace work-area,
and renderer frame from the single `GnomeEngine: wallpaper geometry` log. The
renderer frame must match the monitor rectangle; the work area is diagnostic
only. On X11, `xwininfo` can confirm the mapped surface dimensions and `xprop`
can inspect its EWMH type and absence of strut properties. Do not change panel
or dock settings as part of this test.

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
- [ ] Repeat Apply → Stop → Apply at least three times, including the same
  wallpaper twice; the renderer process and D-Bus service should remain usable
  and every Apply should create a fresh desktop surface.
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

Earlier implementation runs recorded a GNOME 46/Xorg renderer Apply/Stop and
EWMH-property smoke, plus a GNOME 46 nested-Wayland extension/owned-renderer
Apply/Stop smoke. Neither smoke proves full desktop behavior or lifecycle.
The current M7 finalization environment must be recorded separately; do not
carry prior sink, performance, or desktop observations into a new run without
reproducing them there. Mark every unobserved item not tested.

## M6 GUI V1 checklist

The redesigned GUI has previously been launched in a GNOME Shell 46/X11
session for a smoke check. That alone does not complete GUI or M7 acceptance;
review it on each exact M7 target.
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
- [ ] Visit Displays and Settings; confirm no monitor data is fabricated.
- [ ] With the renderer stopped/unavailable, both battery controls remain
  usable, persist across app restart, and do not start a renderer process.
- [ ] Toggle the battery master while playback is active; verify policy takes
  effect immediately. The 20% sub-option is sensitive only while the master is
  enabled.
- [ ] Exercise renderer unavailable/recovery state and About.
- [ ] Review wide, medium/tiled, and narrow layouts, keyboard navigation,
  accessible labels, and the system light/dark schemes.
- [ ] Close the GUI with renderer playback active; verify the app exits without
  calling Stop, then reopen and compare renderer state.

Intentional differences from the HTML: mobile bottom navigation is replaced by
native adaptive split navigation; the Displays page reports only known session
and renderer information because no monitor inventory API exists; the app
uses the system font and color scheme; demo controls, mock wallpapers, and
unsupported settings are omitted. The renderer integration uses
version/backend-specific bridges and still requires the target-session
validation below.

## M5 Ubuntu package acceptance

Build the package on Ubuntu 24.04 amd64 as the ABI-baseline candidate after the
GNOME 46 runtime evidence permits the package dependency to include Shell 46.
Test that same artifact on Ubuntu 24.04 and 26.04. The build script itself
must not use sudo or modify system paths.

- [ ] `./scripts/check-package.sh` passes desktop, AppStream, extension, D-Bus
  activation, version, and SPDX checks.
- [ ] `./scripts/build-deb.sh` builds a
      `dist/gnomeengine_<version>_built-on-ubuntu24.04_amd64.deb` candidate and
      `SHA256SUMS`.
- [ ] `dpkg-deb -I` and `dpkg-deb -c` show the expected package metadata and
  only the app, renderer, desktop data, icon, service, and UUID-matched
  extension; no build tree or Node modules are present.
- [ ] Install the same Noble-built candidate on Ubuntu 24.04 and Ubuntu 26.04;
  verify `gnomeengine` appears in the app grid and `gnome-extensions info`
  recognizes the system extension. Use the explicit maintainer command
  `sudo apt install ./dist/gnomeengine_<version>_built-on-ubuntu24.04_amd64.deb`.
- [ ] Verify the renderer starts through session D-Bus activation only after
  Apply, and that no package script enabled the extension or changed user
  preferences.
- [ ] Remove with `sudo apt remove gnomeengine`; system package files disappear
  while `$XDG_DATA_HOME/gnomeengine` and configuration remain. Reinstall and
  verify the user's Library returns.
- [ ] On Ubuntu 26.04, run APT dependency resolution and install the same Noble
      package; do not treat dependency simulation alone as install/runtime
      acceptance.
- [ ] Run `appstreamcli validate`, `desktop-file-validate`, and `lintian` when
  installed; record unavailable tooling rather than implying it passed.

The current finalization shell has not built or installed the package. Package
build, dependency resolution, installation, GUI/extension operation, removal,
and reinstall remain separate acceptance items; CI package builds alone do
not satisfy them.
