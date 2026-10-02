# M2: Renderer Service and D-Bus Control Plane

## Goal

Turn the existing renderer into a single-instance, event-driven session-bus
service that controls the existing GStreamer playback path through a small
explicit API. Keep video frames and decoding outside D-Bus and GNOME Shell.

## Why this milestone exists

The future desktop application and Shell lifecycle integration need one
reusable control surface. A service also gives playback a clear owner and makes
state and cleanup observable without tying them to a visible app window.

## Current renderer architecture

`renderer/src/main.rs` is currently the only Rust program. It requires a video
path at startup, initializes GStreamer and a GTK application, constructs
`playbin` with `gtk4paintablesink` plus an audio `fakesink`, loops on EOS, and
shows a normal decorated GTK application window. Shutdown moves the pipeline to
GStreamer `NULL`. There is no service, D-Bus code, controller, explicit state
machine, or unit test.

The workspace root includes only `renderer`. Although a `core/` directory is
present, it has no tracked implementation and is not a workspace member. Do not
add shared-core structure without a concrete second consumer.

## Current M1 integration

M1 is not complete. Its active plan is
`docs/plans/active/gnome-background-integration.md`. The current branch adds
only extension metadata and a minimal enable/disable ES-module skeleton. The
renderer still uses application ID `io.github.gnomeengine.Renderer` and opens
an ordinary GTK window. No renderer identity discovery, actor bridge, or
background placement exists; no GNOME runtime behavior has been compiled or
manually validated.

The M2 D-Bus service can be designed independently, but `ApplyVideo` cannot be
accepted as applying a *desktop wallpaper* until the M1 bridge exists. Keep
that acceptance item blocked rather than claiming it through the service API.

## D-Bus APIs researched

The project uses GTK/GLib/GStreamer Rust bindings, so use GIO/GDBus through the
existing gtk-rs dependency graph if the actual crate API and feature set
compile; avoid adding a second D-Bus stack without a concrete limitation.

- GIO documents `g_bus_own_name()` / `g_bus_unown_name()` for asynchronous bus
  name ownership. Its guidance says to export objects in the bus-acquired
  callback, before requesting/acquiring the well-known name, and to clean up if
  ownership is lost.
- GIO's `DBusConnection::register_object()` exports an introspected object and
  supplies the standard Peer, Introspectable, and Properties interfaces.
- The D-Bus API Design Guidelines describe XML interface files as a useful
  machine-readable canonical API, recommend typed D-Bus values, and explain
  `a{sv}` as an extensible dictionary. They recommend replying once to each
  request and representing operation failure with a D-Bus error.
- gtk-rs `gio` provides the Rust bindings for GIO and is maintained as part of
  gtk-rs-core. The repository currently depends on GTK4 0.9 / GLib 0.20, but
  cannot resolve or compile the exact GIO API in this environment.

References:

- [GIO D-Bus name ownership](https://docs.gtk.org/gio/dbus-name-owning.html)
- [GIO `bus_own_name`](https://docs.gtk.org/gio/func.bus_own_name.html)
- [GIO `DBusConnection::register_object`](https://docs.gtk.org/gio/method.DBusConnection.register_object.html)
- [D-Bus API Design Guidelines](https://dbus.freedesktop.org/doc/dbus-api-design.html)
- [gtk-rs GIO crate](https://docs.rs/gio/latest/gio/)

## Proposed interface

The proposed experimental internal interface is:

- Bus name: `io.github.mvk999.GnomeEngine.Renderer`
- Object path: `/io/github/mvk999/GnomeEngine/Renderer`
- Interface: `io.github.mvk999.GnomeEngine.Renderer`
- Methods: `ApplyVideo(s path)`, `Pause()`, `Resume()`, `Stop()`,
  `GetStatus() -> a{sv}`
- Signals: `StateChanged(s state)`, `PlaybackError(s message)`

The XML introspection file should be the single signature source. `GetStatus`
should initially expose only `state`, `currentVideo`, and `lastError`. The
interface is experimental and internal, not a promised stable public API.

Errors should distinguish malformed/unsupported requests, local file
validation, and playback failures without growing an unnecessary error
taxonomy. Only local regular files are accepted. Do not treat an extension as
proof of media type; GStreamer remains responsible for discovering supported
video content. D-Bus requests are serialized through the existing GLib main
context, not a new worker thread or custom event loop.

## State machine

Use one explicit state value: `Stopped`, `Loading`, `Playing`, `Paused`, or
`Error`. `Stop` and repeated pause/resume requests should be safe and idempotent
where possible. New application replaces the current source only after input
validation; an invalid request should not destroy a currently healthy playback.
Pipeline teardown must reach GStreamer `NULL` before ownership is released.

`Stopped` must have no active video pipeline or renderer surface. `Paused` must
set the GStreamer pipeline to a paused state rather than merely freezing its
visible frame. Status signals are emitted only on actual transitions; never
emit per-frame messages.

## Process lifecycle

The renderer owns the session-bus well-known name and runs one GLib main loop
for GIO, GTK, and GStreamer events. Name ownership is the single-instance
mechanism. A competing process exits cleanly without displacing the current
owner. Losing the name or normal shutdown cleans up the pipeline and surfaces.
`Stop()` stops playback but leaves the service available for later requests.
Full D-Bus activation, systemd user units, and packaging are out of scope.

## Implementation slices

1. **Unblock and establish baseline** — provide the documented Rust stable
   toolchain and GTK4/GIO/GStreamer development packages, run canonical checks,
   and record the passing baseline before product code changes.
2. **Pure state model** — add the smallest state transition/controller logic
   with tests for legal transitions, idempotence, errors, and replacement.
3. **IPC contract** — add the introspection XML, validate it in checks, and
   document the experimental method/signal/status contract.
4. **Session-bus ownership** — register/export the object through GIO, handle
   duplicate ownership and name loss, and keep startup idle without a pipeline.
5. **Playback commands** — route Apply/Pause/Resume/Stop to the existing
   GStreamer implementation, with local path validation and deterministic
   cleanup. Preserve the current command-line playback mode only if it can use
   the same controller without duplicating playback logic.
6. **Status, signals, and smoke path** — expose status and transition/error
   signals; document `gdbus` commands and add deterministic checks that do not
   require a full GNOME session.
7. **M1 integration handoff** — validate renderer startup/stop against the real
   Shell background bridge when M1 is complete. Do not count a normal GTK
   window as wallpaper success.

## Testing strategy

- Required baseline command `./scripts/check.sh` was attempted before planning;
  extension syntax passes, but the script exits 127 because Cargo is missing.
- Add focused unit tests for state transitions and status conversion before
  wiring those behaviors into GTK/GStreamer.
- Validate the interface XML with an available parser or GIO itself and add a
  deterministic regression check to the canonical script.
- Use `dbus-run-session` for a service-level smoke test only if the available
  GTK/GIO test dependencies support it without a full compositor; otherwise
  keep the exact `gdbus` manual sequence documented.
- Test actual background Apply/Stop, Alt+Tab, Overview, and workspace behavior
  only on supported GNOME 50+ Wayland with M1 bridge present.

## Security implications

Treat D-Bus calls as external input despite use of the session bus. Accept only
local paths, validate file existence/type/access before disturbing active
playback, use structured file APIs, and never construct shell commands. Keep the
interface small; do not expose arbitrary command execution, unrestricted file
reads, remote URIs, or media bytes. Treat malformed media as an ordinary
playback error and release partial pipelines.

## Performance implications

D-Bus is control plane only: no frames, per-frame statistics, or high-frequency
signals. No renderer pipeline exists while Stopped. Calls and state changes run
on the GLib main context; do not add a thread unless a demonstrated blocking
operation requires it. Measure Playing, Paused, and Stopped later, without
making unmeasured CPU claims.

## Risks

- The current environment cannot compile Rust or native GTK/GIO/GStreamer
  bindings, so an implementation here could not meet the green-baseline rule.
- M1 is incomplete, so a correct service cannot yet make its GTK surface a
  desktop background.
- `ApplyVideo` is asynchronous in effect; the method reply and signals must
  clearly distinguish request acceptance from later playback success/failure.
- GIO callback ownership and name-loss cleanup must be implemented on the
  correct GLib main context and validated against the repository's binding
  versions.
- A GIO crate API that is only transitively present through GTK should not be
  assumed to be directly usable; confirm dependency and feature behavior when
  Cargo is available.

## Acceptance criteria

- [ ] M1 status has been reviewed and is accurately documented.
- [ ] Baseline `./scripts/check.sh` passes before service implementation.
- [ ] A D-Bus XML contract exists and is validated.
- [ ] Renderer owns the session-bus name and prevents a duplicate service.
- [ ] Service starts in `Stopped`, without a video pipeline or surface.
- [ ] ApplyVideo accepts only a valid local file and starts playback.
- [ ] Pause/Resume alter the GStreamer state and are safe when repeated.
- [ ] Stop releases the pipeline/surface but leaves the service callable.
- [ ] GetStatus reports state, current source, and last error.
- [ ] StateChanged and PlaybackError report meaningful transitions/errors.
- [ ] State machine and status conversion have focused unit tests.
- [ ] Invalid input and duplicate process do not crash or displace the service.
- [ ] Repeated replacement/stop does not retain old pipelines.
- [ ] There is no per-frame IPC, polling, network, or extra service process.
- [ ] The real desktop Apply/Stop path is validated after M1 bridge completion.
- [ ] Documentation and smoke commands match the implemented contract.

## Progress

- [x] Repository status, history, branch, remote, and canonical baseline checked.
- [x] Existing docs and renderer/extension state reviewed.
- [x] M1 implementation status reviewed: extension skeleton only; real bridge
  not implemented or runtime-validated; M1 plan remains active.
- [x] GIO/GDBus and D-Bus API guidance researched.
- [ ] Restore a buildable validation environment and get baseline green.
- [ ] Implement and test state model.
- [ ] Define and validate D-Bus introspection contract.
- [ ] Register single-instance session-bus service.
- [ ] Implement playback control/status/signals.
- [ ] Run service and target GNOME integration tests.

## Discoveries

- `./scripts/check.sh` runs the extension syntax check successfully, then exits
  127 with `Rust/Cargo is required`; `cargo`, `rustc`, and `rustup` are absent.
- `pkg-config` cannot find `gio-2.0`, `glib-2.0`, `gtk4`, or `gstreamer-1.0`,
  so native compile/link dependencies are also missing.
- The working desktop remains GNOME Shell 46 on X11, not the GNOME 50+ Wayland
  target; no GUI/Shell acceptance tests can be performed here.
- The renderer GTK application ID is still `io.github.gnomeengine.Renderer`;
  it has not yet been changed to the product's intended ID.
- The existing M1 extension currently has only enable/disable log messages and
  no renderer detection or cleanup state to coordinate with the service.

## Decisions made during implementation

No service implementation decision has been committed. Current design evidence
favors GIO/GDBus on the session bus, a small explicit XML contract, and one
GLib main context. Implementation is gated on restoring the documented build
prerequisites and obtaining a green baseline.

## Post-implementation notes

Pending. Do not move this plan to `completed/` until the relevant automated
criteria pass and the GNOME-dependent Apply/Stop behavior is validated or
explicitly separated as a known remaining milestone.
