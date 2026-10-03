# Desktop Application Design

## Responsibilities

`app/` is a single-instance GTK4/Libadwaita client. It owns the local library,
video import, metadata and thumbnail generation, one on-demand detail preview,
and the user-facing renderer controls. It does not own desktop playback.

```text
GnomeEngine app
  ├── XDG local wallpaper library
  ├── GTK/GStreamer detail preview (only while detail is open)
  └── session D-Bus client
          │
          ▼
     Renderer service ── GStreamer playback
          │
          └── lifecycle state from GNOME extension / system observers
```

The renderer remains independent when the app closes. Closing or quitting the
app never sends `Stop`. An explicit Stop action is the only app action that
stops playback. If there is no renderer owner when Apply is requested, the app
looks for `gnomeengine-renderer` beside itself or on `PATH` and starts it with
GIO's argument-vector API (never through a shell). The app waits for session
D-Bus name ownership and then submits the pending Apply request. A failed
startup is reported in the UI.

## Navigation and preview

Library is the home page: an explanatory empty state or responsive static image
grid, never a video-playing grid. Detail navigation creates one muted `GtkVideo`
preview. Back, Import navigation, and explicit app teardown stop and release
that preview. Preview and renderer pipelines are intentionally separate.

The header reflects renderer availability/state and pause reasons. A current
library item is marked active only after `GetStatus` confirms the renderer's
canonical content path. Preferences currently expose only the M3 battery pause
policy; it is stored by the renderer under the user's XDG config directory so
it remains effective with the GUI closed. About does not claim an undeclared
license.

## Renderer client and errors

`RendererClient` watches the renderer well-known name and subscribes to
`StateChanged`, `PlaybackError`, `PauseReasonsChanged`, and `PolicyChanged`.
On owner appearance it subscribes first, then asynchronously queries
`GetStatus`. Calls are asynchronous and there is no periodic status polling.
Service disappearance resets the UI to unavailable; reappearance triggers a
new snapshot. D-Bus and media failures are logged and surfaced as concise UI
feedback.

## Limits

The current M1 renderer still displays a normal GTK preview window; it does not
place its surface on GNOME's desktop background layer. Therefore this app
controls the renderer prototype but cannot yet provide the final real-wallpaper
experience. GNOME 50+ Wayland runtime validation was not available on the host
used during M4 implementation.
