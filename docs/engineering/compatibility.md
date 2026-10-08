# Platform Compatibility

GnomeEngine's requested M7 target matrix is:

| Ubuntu | GNOME Shell | Session | GUI | Renderer | Shell bridge | Lifecycle | Package |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| 24.04 LTS | 46 | Wayland | Build validated; runtime not tested | Nested Apply/Stop smoke passed; visual desktop behavior not validated | Extension loaded and owned-renderer D-Bus smoke passed; desktop semantics not fully validated | Not runtime tested | Not tested |
| 24.04 LTS | 46 | X11/Xorg | Build validated; runtime not tested | Apply/Stop and EWMH hints verified on real host; visual/lifecycle acceptance open | Renderer surface verified; Shell extension handoff not tested | Not runtime tested | Not tested |
| 26.04 LTS | 50 | Wayland | Not tested here | Not tested here | Not runtime tested | Not runtime tested | Not tested |

“Build validated” means `./scripts/check.sh` compiled and tested the workspace
against the installed Noble native development libraries. It does not mean the
application or extension has been run as a wallpaper. The current local host
is Ubuntu 24.04.5 / GNOME Shell 46 / `XDG_SESSION_TYPE=x11`; the extension is
not installed in that session. GNOME 46 Wayland uses a disposable nested
session for the limited smoke test below; GNOME 50 Wayland requires a separate
graphical test environment.

The Noble compile baseline observed on 2026-10-03 is GTK 4.14.5, Libadwaita
1.5.0, and GStreamer 1.24.2. Rust binding feature levels are GTK 4.12 and
Libadwaita 1.4. The renderer release binary builds on this host and has no
unresolved shared libraries according to `ldd`. It statically registers the
GTK4 GStreamer sink with Wayland EGL, X11 EGL, X11 GLX, and GTK 4.14 DMA-BUF
features. The GNOME46/X11 path now configures `_NET_WM_WINDOW_TYPE_DESKTOP`,
sticky/skip-taskbar/skip-pager state, and all-workspaces on the GDK X11 surface
before map. The GNOME46/Wayland path uses a Shell-owned `Meta.WaylandClient`
child because Mutter 14's GJS typelib exposes `spawnv()`/`make_desktop()` but
does not expose the native `new_indirect()` or `setup_fd()` functions. In an
isolated GNOME 46 nested Wayland session, the extension exported its startup
coordinator, started the renderer, and D-Bus `ApplyVideo`/`Stop` transitioned
the renderer through `playing` and `stopped`; the private `gtk4paintablesink`
was selected. This smoke test does not validate visible desktop composition,
Alt+Tab/Overview exclusion, workspace behavior, input pass-through, lock,
suspend, or power-management lifecycle. The renderer control D-Bus API is
unchanged. The package and extension metadata have not been expanded to claim
compatibility.

On the real GNOME46/Xorg host, the renderer also completed D-Bus Apply/Stop.
`xprop` observed `_NET_WM_WINDOW_TYPE_DESKTOP`, sticky/skip-taskbar/skip-pager,
all-workspaces, and `WM_HINTS` with keyboard input disabled. GTK rewrote the
focus hint while mapping, so the renderer reapplies that non-focusable hint
immediately after `present()`; the EWMH window-type and workspace/taskbar hints
are still set before map. This did not run the Shell extension's X11 discovery
or test Alt+Tab, Overview, workspaces, input pass-through, lock, or suspend.

For X11 validation, verify `echo "$XDG_SESSION_TYPE"` returns `x11` after
logging into “Ubuntu on Xorg”. A Wayland session running X11 clients through
XWayland is still a Wayland test, not an X11 test.

## Finalization environment check (2026-10-08)

The current command environment is Ubuntu 24.04.5 and reports
`XDG_SESSION_TYPE=x11`; the installed `gnome-shell` binary reports 46.0.
However, there is no running `gnome-shell` process and the session manager bus
is inaccessible, so this is not a usable live GNOME X11 test session. It has
GTK 4.14.5, Libadwaita 1.5.0, and GStreamer 1.24.2 runtime packages, but no
`cargo` or `rustc`, no GTK/Libadwaita/GStreamer development pkg-config files,
and no installed GnomeEngine package. A temporary metadata-adjusted extension
bundle could be created, but Shell could not discover or enable it; the test
copy was removed. The extension syntax and
`./scripts/check-package.sh` pass; `./scripts/check.sh` and
`./scripts/build-deb.sh` stop because Cargo is unavailable. No package build,
installation, app launch, renderer playback, or live Shell integration result
was produced in this environment. The earlier Noble build and limited smoke
results above remain historical evidence and were not reproduced here.

APT candidate inspection found the declared `gnome-shell (>= 50)` dependency
cannot be satisfied by this Noble host, whose GNOME Shell candidate is 46.0.
The GTK media backend and declared GStreamer plugin packages have Noble
candidates, while `gstreamer1.0-gtk4` has no Noble candidate and is not a
package dependency. Without a built `.deb`, this was not a package-level APT
simulation. No Ubuntu 26.04 APT or installation result was available.
