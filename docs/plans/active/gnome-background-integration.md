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

- The workspace contains the existing `renderer` crate only.
- The renderer uses GTK4, GStreamer `playbin`, `gtk4paintablesink`, a fakesink
  for audio, EOS seek-to-start looping, and a normal decorated GTK window.
- No GNOME Shell extension or renderer-to-Shell protocol exists.
- `./scripts/check.sh` baseline is blocked because Rust/Cargo is not installed.
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

Proceed incrementally. First add a minimal GNOME 50 ES-module extension skeleton
with symmetric enable/disable cleanup. Give the renderer a stable GTK
application ID and use event-driven `Meta.Display::window-created` discovery,
including one initial window enumeration on enable. Keep a GNOME background
actor bridge behind a narrowly scoped integration module.

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
- The local desktop is GNOME 46 on X11, and Rust/GStreamer development tooling is
  missing; renderer compilation and target-session checks are unavailable here.
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
   windows at enable, wait for actor readiness using lifecycle signals, and
   disconnect every signal on disable.
4. **Background bridge** — after GNOME 50 API verification, attach/clone only
   the renderer actor into the appropriate background layer; prevent focus and
   input; handle monitor topology without assuming monitor 0.
5. **Lifecycle hardening** — renderer close/crash, extension reload, monitor
   changes, and safe restoration of the ordinary static background.
6. **Documentation and validation** — record chosen Shell API and limitations;
   complete the manual GNOME checklist; move this plan to completed only when
   the acceptance criteria have been validated.

## Testing strategy

- Run `./scripts/check.sh` before and after applicable slices. Its Rust baseline
  currently cannot run because Cargo is absent; do not report it as passing.
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
- [ ] Renderer is identified by its stable GTK application ID.
- [ ] Discovery is signal-driven and handles renderer-before-extension startup.
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
- [x] Baseline validation attempted; blocked by missing Rust/Cargo.
- [x] GNOME/Mutter API research recorded.
- [x] Add extension lifecycle skeleton.
- [ ] Set stable renderer GTK application ID.
- [ ] Implement event-driven renderer window discovery.
- [ ] Establish and validate GNOME 50 background actor integration.
- [ ] Complete target-session manual checks.

## Discoveries

- Public Mutter APIs support event-driven top-level window discovery and actor
  lookup, but the reviewed documentation does not expose a public operation to
  make an external surface a GNOME desktop background.
- Existing reference behavior suggests private Shell background hooks may be
  involved. This is a critical compatibility risk, not yet a chosen runtime
  implementation.
- The current development environment cannot validate the milestone's target
  platform; a GNOME 50+ Wayland session and Rust toolchain are required for final
  acceptance.
- The extension skeleton uses GNOME's ES-module `Extension` lifecycle and is
  syntax-checked through the canonical repository check script; it performs no
  shell work while idle.
