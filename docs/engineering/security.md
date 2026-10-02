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

## IPC and future renderers

- Keep D-Bus methods few, typed, and scoped to wallpaper IDs or validated
  managed resources; do not expose arbitrary file reads or shell execution.
- Keep video frames off D-Bus.
- Web wallpapers must be offline by default, isolated from arbitrary host file
  access and command execution, and loaded only when needed.
- A future network catalog must validate downloaded packages and preserve the
  local-only behavior of installed wallpapers.

## Reporting

Do not log private file contents or personal data. Keep diagnostics local; the
project currently has no telemetry or automatic crash upload.
