#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(dpkg --print-architecture)" != "amd64" ]]; then
  printf '%s\n' 'error: this upstream package currently targets amd64 only' >&2
  exit 2
fi

for tool in cargo dpkg-buildpackage dpkg-deb dpkg-parsechangelog dh fakeroot; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'error: %s is required; install the documented package build dependencies first\n' "$tool" >&2
    exit 127
  fi
done

./scripts/check-package.sh

dpkg-buildpackage --build=binary --no-sign --root-command=fakeroot

deb_version="$(dpkg-parsechangelog --show-field Version)"
architecture="$(dpkg --print-architecture)"
built_package="$(dirname "$repo_root")/gnomeengine_${deb_version}_${architecture}.deb"
destination="$repo_root/dist/$(basename "$built_package")"

if [[ ! -f "$built_package" ]]; then
  printf 'error: expected package was not produced: %s\n' "$built_package" >&2
  exit 1
fi

mkdir -p "$repo_root/dist"
install -m 0644 "$built_package" "$destination"

printf '%s\n' '==> Inspect package metadata and contents'
dpkg-deb --info "$destination"
package_name="$(dpkg-deb --field "$destination" Package)"
package_version="$(dpkg-deb --field "$destination" Version)"
package_architecture="$(dpkg-deb --field "$destination" Architecture)"
package_dependencies="$(dpkg-deb --field "$destination" Depends)"
if [[ "$package_name" != gnomeengine || "$package_version" != "$deb_version" || "$package_architecture" != amd64 ]]; then
  printf 'error: unexpected package identity (%s %s %s)\n' \
    "$package_name" "$package_version" "$package_architecture" >&2
  exit 1
fi
for dependency in 'gnome-shell (>= 50)' libgtk-4-media-gstreamer \
  gstreamer1.0-gtk4 gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
  gstreamer1.0-libav; do
  if ! grep -Fq "$dependency" <<<"$package_dependencies"; then
    printf 'error: package is missing required runtime dependency %s\n' "$dependency" >&2
    exit 1
  fi
done
contents="$(dpkg-deb --contents "$destination")"
for expected in \
  './usr/bin/gnomeengine' \
  './usr/libexec/gnomeengine/gnomeengine-renderer' \
  './usr/share/applications/io.github.mvk999.GnomeEngine.desktop' \
  './usr/share/metainfo/io.github.mvk999.GnomeEngine.metainfo.xml' \
  './usr/share/icons/hicolor/scalable/apps/io.github.mvk999.GnomeEngine.svg' \
  './usr/share/gnomeengine/brand/gnomeengine-logo.svg' \
  './usr/share/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service' \
  './usr/share/doc/gnomeengine/copyright' \
  './usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/extension.js' \
  './usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/metadata.json'; do
  if ! grep -Fq "$expected" <<<"$contents"; then
    printf 'error: package is missing expected path %s\n' "$expected" >&2
    exit 1
  fi
done
if grep -Eq '(^|/)(target|node_modules)/|/home/|\.ts$' <<<"$contents"; then
  printf '%s\n' 'error: package contains development-only paths or TypeScript sources' >&2
  exit 1
fi

printf 'built %s (%s bytes)\n' "$destination" "$(stat -c '%s' "$destination")"
