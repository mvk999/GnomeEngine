# Local Wallpaper Library

## Storage layout

The library lives below GLib's XDG user data directory:

```text
$XDG_DATA_HOME/gnomeengine/wallpapers/<uuid>/
├── manifest.json
├── thumbnail.png
└── content/wallpaper.<original-extension>
```

Unset `XDG_DATA_HOME` uses the GLib/XDG fallback (normally
`~/.local/share`). Import creates a managed copy; the source file is not
modified or removed. Re-importing the same file creates another independent
item. There is no database and no background rescan.

## Manifest schema v1

```json
{
  "schemaVersion": 1,
  "id": "<uuid>",
  "title": "Example",
  "type": "video",
  "createdAt": "<UTC timestamp>",
  "content": { "entry": "content/wallpaper.mp4" },
  "media": {
    "width": 1920,
    "height": 1080,
    "fps": 30.0,
    "durationSeconds": 24.3,
    "codec": "video/x-h264"
  }
}
```

Media values are optional and recorded only when GStreamer discovery provides
them. The entry path must be a non-empty relative path contained in the
wallpaper directory. Unknown schema versions, invalid IDs/types, symlinked
manifests/content, and paths escaping the managed directory are rejected.
Invalid items are skipped individually so one damaged folder does not stop the
rest of the library from opening.

## Import transaction

1. The user chooses one local file through GTK's asynchronous file dialog.
2. The import worker requires a regular readable file and discovers a video
   stream with GStreamer.
3. It creates an app-owned `.import-<uuid>` directory, copies the original bytes,
   records available metadata, and attempts one bounded PNG thumbnail.
4. It writes the manifest and atomically renames the completed directory to its
   UUID. A failed operation cleans only its own staging directory. Startup
   removes only real directories with the reserved `.import-` prefix.

Thumbnail generation is best-effort: a valid video remains importable when
thumbnail decoding fails. The library shows a placeholder instead. Startup
loads manifests and cached thumbnails only; it does not rediscover media or
regenerate frames.

## Removal

Removal requires confirmation and deletes only the managed UUID directory. If
that exact media path is active, the app first asks the renderer to Stop and
deletes only after the D-Bus call succeeds. It never deletes the original
source. A filesystem or stop failure leaves the library entry intact.
