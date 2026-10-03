# GNOME Background Integration

## Goal

Prove that the existing Rust/GStreamer video renderer can be presented as a real
GNOME desktop background on GNOME 50+ under Wayland, while normal windows,
input, workspaces, Overview, and application switching continue to work.

## Why

This is the first vertical product slice and the main technical risk before
building the desktop application. Media decoding stays outside GNOME Shell; the
extension is limited to compositor integration and lifecycle events.

## Current state

- The workspace contains a Rust `renderer` crate and the ES-module Shell
  extension. The renderer now has a guarded Mutter desktop-window bridge, but
  target-session behavior has not been runtime-validated.
- The renderer prefers GTK4, GStreamer `playbin`, and `gtk4paintablesink`, with
  GTK `GtkVideo` fallback when the optional sink plugin is absent; audio is
  muted and playback loops in a normal GTK window.
- The extension reports fullscreen, session lock, monitor topology, and
  built-in-panel power lifecycle state over D-Bus. It now discovers the
  renderer by GTK application ID, classifies it as a Mutter `DESKTOP` window,
  and sizes it to the primary monitor on topology changes.
- `./scripts/check.sh` passes after Rust/native development dependencies were
  installed; the GStreamer GTK paintable plugin is unavailable, so the new GTK
  media backend fallback is selected in code but still needs runtime playback
  validation on this host.
- The available test desktop is Ubuntu 24.04, GNOME Shell 46, X11. It cannot
  validate the GNOME 50+ Wayland behavior required by this milestone.

## GNOME APIs researched

Research was performed against official GNOME Shell/Mutter API documentation
currently generated for Mutter 51, plus the official GNOME Shell 45 extension
module guide. This is adjacent current documentation, not a substitute for
testing the target GNOME 50 runtime.

- `Meta.Display::window-created` provides event-driven discovery; `list_all_windows`
  can cover a renderer that existed before the extension enabled.
- `Meta.Window::gtk-application-id` / `get_gtk_application_id()` provides a
  stronger identity than a user-visible title. `get_compositor_private()` is
  the documented route to the compositor actor after it exists.
- `Meta.Window.hide_from_window_list()` is documented for hiding from window
  lists such as taskbars and pagers. The documentation does not establish that
  this alone excludes a window from GNOME Shell Alt+Tab and Overview.
- `Meta.WindowActor` is the compositor actor for a top-level window and exposes
  the associated `Meta.Window` and texture. Its API documentation does not
  describe an extension-facing API for inserting an external surface into the
  desktop background group.
- The GNOME Shell extension guide uses ES modules with a default-exported
  `Extension` subclass and reversible `enable()` / `disable()` lifecycle.
- `Meta.Display` exposes monitor geometry and primary-monitor information;
  discovery should react to monitor changes rather than poll.

References:

- [Mutter Meta.Display](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Display.html)
- [Mutter Meta.Window](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.Window.html)
- [Mutter Meta.WindowActor](https://gnome.pages.gitlab.gnome.org/mutter/meta/class.WindowActor.html)
- [GNOME Shell extensions in GNOME 45](https://blogs.gnome.org/shell-dev/2023/09/02/extensions-in-gnome-45/)
- Hanabi was reviewed only as a behavioral/technical reference. Its current
  implementation uses private Shell hooks and polling in some paths; no code is
  copied or adopted from it.

## Possible approaches

1. Keep the renderer as an ordinary top-level GTK window. This is already the
   prototype behavior, but does not establish background stacking, input
   pass-through, Alt+Tab/Overview exclusion, or workspace semantics.
2. Use documented Meta window discovery and compositor actor access, then attach
   or clone the actor into a Shell background actor. The discovery pieces are
   documented, but a supported public API for background insertion was not
   found.
3. Hook GNOME Shell's private background/window management internals and use
   compositor actors/clones. This may enable the desired visual result, but is
   version-sensitive and must be tested against GNOME 50 before it is accepted.
4. Change GNOME's static wallpaper setting or use X11 window hints/tools. These
   violate the product requirements and are rejected.

## Chosen approach

Use a minimal GNOME 50 ES-module extension with symmetric enable/disable
cleanup. The renderer has a stable GTK application ID; the extension discovers
its surface through event-driven `Meta.Display::window-created` handling plus
one initial window enumeration on enable. The current experimental bridge
classifies that top-level with `Meta.Window.set_type(Meta.WindowType.DESKTOP)`;
it does not reparent or clone compositor actors. This classification remains
unaccepted until the target Wayland behavior is manually validated.

No private Shell hook will be treated as accepted until its exact GNOME 50 API
is inspected and the behavior is runtime-validated in a GNOME 50+ Wayland
session. The current environment cannot provide that validation. If a private
hook proves necessary, record its version risk and isolate it so the extension
can cleanly fall back without affecting ordinary windows or Shell stability.

The extension will remain JavaScript ES modules for the initial lifecycle and
discovery slices. The repository has no TypeScript compiler, GJS type package,
or extension build tool installed; introducing that toolchain for a small
Shell-side bridge would add setup before it provides useful type coverage.
Revisit TypeScript when the extension logic has enough surface to justify it.

## Risks and limitations

- Actual background insertion and reliable Alt+Tab/Overview exclusion may only
  be possible through private Shell internals.
- Mutter 51 documentation is not proof that GNOME 50 has identical behavior.
- The local desktop is GNOME 46 on X11, below the GNOME 50+ Wayland target.
  Mutter fullscreen/topology signals were locally introspected, but renderer
  compilation does not validate background placement or target-session APIs.
- Window actor availability is asynchronous; no fixed sleeps will be used as a
  synchronization mechanism.
- Actor ownership, input transparency, workspace membership, monitor scaling,
  and extension disable cleanup require runtime verification.

## Implementation slices

1. **Extension skeleton** — metadata targets GNOME 50; ES-module lifecycle logs
   enable/disable; syntax validation; no Shell work when idle.
2. **Renderer identity** — set GTK application ID to
   `io.github.mvk999.GnomeEngine.Renderer`; verify identity using Meta APIs, not
   the title.
3. **Event-driven discovery** — connect `window-created`, inspect existing
   windows at enable, and disconnect per-window signals on teardown. Done in
   the current implementation.
4. **Background bridge** — use guarded `Meta.WindowType.DESKTOP` classification,
   an integration-ready D-Bus handshake, a non-focusable GTK surface, and an
   empty input region. Implemented; target runtime validation remains open.
5. **Lifecycle hardening** — basic fail-closed teardown and topology
   recalculation are implemented; renderer crash/reload and target monitor
   behavior still require manual validation.
6. **Documentation and validation** — record chosen Shell API and limitations;
   complete the manual GNOME checklist; move this plan to completed only when
   the acceptance criteria have been validated.

## Testing strategy

- Run `./scripts/check.sh` before and after applicable slices. It now passes in
  the available environment; do not treat that as GNOME runtime validation.
- Run available deterministic syntax checks for extension JavaScript.
- Add automated tests only for separable pure logic; do not mock the entire
  GNOME Shell runtime.
- Perform manual integration checks on GNOME 50+ Wayland, preferably nested or
  development session, using the checklist in
  `docs/engineering/manual-testing.md`.
- Do not claim desktop integration works until video visibility, Alt+Tab,
  Overview, input pass-through, workspaces, stop/restore, and extension reload
  have been checked in that target session.

## Performance implications

The extension must be event-driven and idle without timers, frame work, media
decoding, or repeated library scans. Video decoding remains in the Rust renderer.
Pause/visibility behavior beyond what is necessary to prove background placement
is out of scope for this milestone.

## Security implications

The extension will match only the fixed renderer application ID and will never
execute commands or trust window titles as paths or commands. It will not alter
the user's GNOME background settings. Any Shell-private API use must be isolated
and guarded so failures cannot modify ordinary application actors.

## Acceptance criteria

- [ ] Extension targets GNOME 50+ and has a reversible enable/disable lifecycle.
- [x] Renderer is identified by its stable GTK application ID.
- [x] Discovery is signal-driven and handles renderer-before-extension startup.
- [ ] Renderer video appears as the desktop background, with no normal window.
- [ ] Renderer is absent from Alt+Tab and Overview and receives no focus/input.
- [ ] Panel, notifications, normal windows, and workspace switching work.
- [ ] Stopping the renderer reveals the user's unchanged static background.
- [ ] Multiple monitors do not crash or place the surface unpredictably.
- [ ] Extension reload leaves no duplicate actors or connected signals.
- [ ] Checks pass where available; unavailable target/runtime checks are clearly
  reported rather than inferred.

## Progress

- [x] Repository and existing renderer inspected.
- [x] Rust/GStreamer renderer now builds and canonical checks pass.
- [x] Renderer lifecycle D-Bus protocol exists.
- [x] GNOME/Mutter API research recorded.
- [x] Add extension lifecycle skeleton.
- [x] Set stable renderer GTK application ID and make its surface input-transparent.
- [x] Implement event-driven renderer discovery and Mutter desktop classification.
- [ ] Validate GNOME 50+ Wayland background stacking and recovery behavior.
- [ ] Complete target-session manual checks.

## Discoveries

- Mutter documents `Meta.Window.set_type()` and `Meta.WindowType.DESKTOP`. The
  implementation avoids private Shell actor reparenting, but changing a
  Wayland toplevel's type remains version-sensitive and needs target testing.
- GDK's input-region API prevents the wallpaper surface from receiving pointer
  input; Apply fails closed when the active backend reports no input-shape
  support.
- The readiness handshake prevents Apply from falling back to an ordinary
  visible GTK player when the extension is absent.
- The current development environment cannot validate the milestone's target
  platform; a GNOME 50+ Wayland session and Rust toolchain are required for final
  acceptance.
- The package was unpacked with `dpkg-deb -x` for a smoke test, not installed.
  Files extracted under a temporary directory are not thereby registered with
  the already-running GNOME Shell. The host is GNOME 46/X11 and remains outside
  the chosen support target; Apply correctly fails closed there. The app now
  reports this platform mismatch rather than incorrectly telling the user only
  to enable an extension.
- The extension skeleton uses GNOME's ES-module `Extension` lifecycle and is
  syntax-checked through the canonical repository check script; it performs no
  shell work while idle.
