#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

for tool in desktop-file-validate appstreamcli node dpkg-parsechangelog; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'error: %s is required for package metadata validation\n' "$tool" >&2
    exit 127
  fi
done

printf '%s\n' '==> Desktop entry'
desktop-file-validate data/io.github.mvk999.GnomeEngine.desktop

printf '%s\n' '==> AppStream metadata'
appstreamcli validate --no-net --strict data/io.github.mvk999.GnomeEngine.metainfo.xml

printf '%s\n' '==> GNOME Shell extension metadata'
node --input-type=module <<'NODE'
import fs from 'node:fs';

const metadata = JSON.parse(fs.readFileSync('extension/metadata.json', 'utf8'));
if (metadata.uuid !== 'gnomeengine@mvk999.github.io') {
    throw new Error('GNOME Shell extension UUID does not match its installation directory');
}
if (JSON.stringify(metadata['shell-version']) !== JSON.stringify(['50'])) {
    throw new Error('The packaged extension must declare only the validated target GNOME Shell version');
}
if (!fs.existsSync('extension/extension.js')) {
    throw new Error('Compiled extension runtime file is missing');
}
NODE
node --input-type=module --check < extension/extension.js

printf '%s\n' '==> Renderer D-Bus activation file'
grep -Fx 'Name=io.github.mvk999.GnomeEngine.Renderer' data/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service
grep -Fx 'Exec=/usr/libexec/gnomeengine/gnomeengine-renderer' data/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service

printf '%s\n' '==> Package version consistency'
workspace_version="$(awk '/^\[workspace\.package\]$/ { in_section=1; next } /^\[/ { in_section=0 } in_section && /^version = / { gsub(/"/, "", $3); print $3; exit }' Cargo.toml)"
package_version="$(dpkg-parsechangelog --show-field Version | sed 's/-[0-9][A-Za-z0-9.+~]*$//')"
appstream_version="$(sed -n 's/.*<release version="\([^"]*\)".*/\1/p' data/io.github.mvk999.GnomeEngine.metainfo.xml | head -n 1)"
if [[ -z "$workspace_version" || "$workspace_version" != "$package_version" || "$workspace_version" != "$appstream_version" ]]; then
  printf 'error: version mismatch (workspace=%s debian=%s appstream=%s)\n' \
    "$workspace_version" "$package_version" "$appstream_version" >&2
  exit 1
fi
printf 'version %s\n' "$workspace_version"

printf '%s\n' '==> Project license consistency'
for file in Cargo.toml debian/copyright; do
  if ! grep -Fq 'GPL-3.0-or-later' "$file"; then
    printf 'error: project SPDX license is missing from %s\n' "$file" >&2
    exit 1
  fi
done
grep -Fq '<project_license>GPL-3.0-or-later</project_license>' data/io.github.mvk999.GnomeEngine.metainfo.xml
grep -Fq '<metadata_license>CC0-1.0</metadata_license>' data/io.github.mvk999.GnomeEngine.metainfo.xml
grep -Fq 'license.workspace = true' app/Cargo.toml
grep -Fq 'license.workspace = true' renderer/Cargo.toml
test -s LICENSE
test -s app/resources/brand/gnomeengine-logo.svg
grep -Fq 'gnomeengine-logo.svg' debian/gnomeengine.install
