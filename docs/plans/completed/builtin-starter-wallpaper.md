# Built-in Starter Wallpaper — ExecPlan

## Goal

Prepare the shared library model, native Library UI, and Debian packaging for
one offline built-in wallpaper. The actual video and thumbnail will be
provided later; no placeholder media is added.

## Supported platform context

This is data/library/package work and adds no GNOME, GTK, renderer, or D-Bus
backend behavior. It preserves the active Ubuntu 24.04/26.04 compatibility
work; runtime support claims remain governed by that plan.

## Current state and discoveries

- User wallpapers live under GLib's XDG user data directory and use manifest
  schema version 1, ID-named directories, and `thumbnail.png` by default.
- Debian packaging stages app files in `debian/rules`; there are no real
  built-in media assets yet.
- The live library reload path runs after import/removal, so it preserves
  configured built-in roots rather than reverting to user-only data.
- The generated package currently contains an empty wallpaper data directory,
  not the incomplete starter manifest.

## Design and implementation

- One `Wallpaper` model carries repository-assigned `WallpaperOrigin`.
- Built-ins are discovered below development assets and XDG system data roots;
  user wallpapers remain in the existing XDG user directory.
- Development assets are considered before system roots. The first occurrence
  of a stable wallpaper ID wins, preventing a user directory from shadowing an
  official ID.
- Built-in data is read-only. Content and optional thumbnail paths are
  relative, regular files, and containment-checked.
- Optional manifest metadata/thumbnail fields are backward-compatible with
  existing schema-v1 imports; old imports continue to default to
  `thumbnail.png`.
- The Library shows `Incluídos com GnomeEngine` and `Seus wallpapers` sections
  only when each source has content. Search is shared across sections, and
  built-in detail does not expose removal.
- Debian staging packages only complete wallpaper directories. A strict
  release gate requires complete media and author/copyright/license metadata.

## Validation and acceptance

- [x] Old user manifests continue to load; user items retain origin and
  deletion behavior.
- [x] Fixture in a named built-in directory loads by stable manifest ID,
  carries `BuiltIn` origin, survives reload, and cannot be removed through
  `Library::remove`.
- [x] Duplicate IDs prefer the earlier built-in source; malformed/incomplete
  entries do not prevent library loading.
- [x] Missing video/thumbnail is a valid development state; no media placeholder
  is committed or packaged.
- [x] A complete future directory is staged under
  `/usr/share/gnomeengine/wallpapers/<name>/` without Rust source changes.
- [x] `./scripts/check.sh` passed (32 app tests and 24 renderer tests).
- [x] `./scripts/check-package.sh` passed.
- [x] `./scripts/build-deb.sh` built and inspected the Ubuntu 26.04 amd64
  package; no wallpaper media was included because none exists yet.
- [x] Strict release validation rejects the incomplete starter directory.
- [ ] Runtime preview/apply validation remains pending until the maintainer
  provides the real video and thumbnail.

## Decisions

- No real video or thumbnail is fabricated or downloaded.
- The existing schema version remains 1; new optional fields deserialize
  backward-compatibly.
- Active renderer path matching remains path-based and applies to both origins.
- Public distribution requires explicit asset rights; the media license is not
  inferred from the application license.
