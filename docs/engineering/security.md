# Security Baseline

GnomeEngine is a local desktop application, but local input is not automatically
trusted. Apply these constraints as features are added.

## Current video input

- Accept files selected by the user; do not invoke scripts or commands from
  media metadata.
- Validate media through the media stack and handle malformed/oversized input
  as an ordinary error.
- Avoid shell command construction. Pass paths as structured arguments or
  file/URI objects.

## Manifests and filesystem

- Validate schema versions, IDs, types, and relative content paths.
- Reject absolute paths, parent traversal, and symlinks that resolve outside
  the managed wallpaper directory.
- Bound media, manifest, and thumbnail sizes before expensive processing.
- Treat imported package contents as untrusted. A future archive extractor
  must prevent Zip Slip and symlink traversal.
- M4 copies user-selected videos into UUID-named managed directories, stages
  import under a reserved app-owned prefix, bounds/parses manifests, rejects
  symlinked content, and verifies canonical content paths remain in that
  directory. Removal validates the manifest/ID and deletes only the managed
  directory, never the original source.

## IPC and future renderers

- Keep D-Bus methods few, typed, and scoped to wallpaper IDs or validated
  managed resources; do not expose arbitrary file reads or shell execution.
- Keep video frames off D-Bus.
- The app's renderer fallback starts only the fixed `gnomeengine-renderer`
  executable via GIO argv (sibling path or PATH lookup); it never interpolates
  a media path into a command or invokes a shell. The selected media path is
  passed as a typed D-Bus string to the renderer's existing file validation.
- Web wallpapers must be offline by default, isolated from arbitrary host file
  access and command execution, and loaded only when needed.
- A future network catalog must validate downloaded packages and preserve the
  local-only behavior of installed wallpapers.

## Reporting

Do not log private file contents or personal data. Keep diagnostics local; the
project currently has no telemetry or automatic crash upload.
