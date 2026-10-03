# Built-in wallpapers

## Purpose

Official wallpapers ship as read-only local data with the Debian package. They
use the same manifest parser, `Wallpaper` model, preview, and renderer path as
user imports. This module has no network access and never copies system media
into a user's data directory.

## Sources and precedence

The library combines:

- built-ins discovered below each `$XDG_DATA_DIRS/gnomeengine/wallpapers`;
  when `XDG_DATA_DIRS` is unset, the XDG defaults `/usr/local/share` and
  `/usr/share` are used;
- the development tree's `assets/wallpapers`, found relative to the running
  executable when it is inside a Cargo repository;
- the user's `$XDG_DATA_HOME/gnomeengine/wallpapers` (GLib supplies the XDG
  default, normally `~/.local/share`).

Development assets are considered before system data roots. The first
occurrence of a stable wallpaper ID wins, so a user directory cannot shadow an
official ID. Invalid built-in entries are logged and do not prevent valid
user wallpapers from loading. Incomplete source-checkout entries are ignored.

## Layout and manifest

Each built-in is an immediate subdirectory, for example:

```text
assets/wallpapers/starter/
├── manifest.json
├── thumbnail.webp
└── wallpaper.mp4
```

The directory name is not the wallpaper ID. `manifest.json` uses the existing
version-1 schema with additive optional `author`, `copyright`, `license`, and
`thumbnail` fields. Older user manifests remain valid; absent `thumbnail`
continues to mean `thumbnail.png`. Both `content.entry` and `thumbnail` are
relative paths and are checked to remain within their wallpaper directory.

Use a stable ID such as `gnomeengine-starter-001`; do not derive it from a
title or filename. New built-in folders can be added without Rust changes when
they contain a valid manifest and all referenced local files.

## Origin and permissions

Origin is assigned by the repository that discovers the wallpaper, never by
manifest data. `BuiltIn` and `User` share one `Wallpaper` type. Built-ins can
be previewed, applied, and shown as active, but cannot be renamed, overwritten,
or deleted. Removal is rejected in the library layer as well as hidden in the
UI. User wallpapers retain their existing import and removal behavior.

Renderer calls receive the canonical path to the packaged video directly,
normally below `/usr/share/gnomeengine/wallpapers`; there is no home-directory
copy. Package upgrades replace system assets in place. If an active video is
replaced during an upgrade, the updated bytes are guaranteed to be used on the
next renderer pipeline reload; package scripts do not restart the renderer.
Removing the package removes package-owned built-ins while preserving user
wallpapers and settings.

## Packaging and validation

Debian staging includes a wallpaper directory only when its manifest and all
referenced regular files exist. Files are installed read-only for normal
users under `/usr/share/gnomeengine/wallpapers/<directory>/`. The package
validation script checks JSON/schema compatibility, stable unique IDs, safe
relative paths, regular files, and containment. Set
`GNOMEENGINE_REQUIRE_STARTER_ASSETS=1` for a release build to fail if no
complete built-in assets are present; ordinary development and CI builds may
omit the not-yet-created media.

## Asset rights

Each public-release wallpaper must have its actual title, author, copyright
holder, and license recorded in its manifest, and distribution rights must be
confirmed. The source-code GPL license does not automatically apply to media.
The starter video and thumbnail are not yet present, so no media license is
asserted here.

## Adding an official wallpaper

Create a folder under `assets/wallpapers/` containing a manifest, thumbnail,
and video; give it a unique permanent ID and complete rights metadata. Then
run `./scripts/check.sh` and `./scripts/check-package.sh`, build the `.deb`, and
inspect it with `dpkg-deb -c`. No application source change is needed.
