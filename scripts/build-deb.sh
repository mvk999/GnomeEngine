#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(dpkg --print-architecture)" != "amd64" ]]; then
  printf '%s\n' 'error: this upstream package currently targets amd64 only' >&2
  exit 2
fi

for tool in cargo rustc dpkg-buildpackage dpkg-deb dpkg-parsechangelog dh fakeroot ldd sha256sum; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'error: %s is required; install the documented package build dependencies first\n' "$tool" >&2
    exit 127
  fi
done

./scripts/check-package.sh

build_dependency_flags=()
if unmet_build_dependencies="$(dpkg-checkbuilddeps 2>&1)"; then
  :
elif [[ "$unmet_build_dependencies" == *'Unmet build dependencies: cargo rustc'* ]] \
  && command -v cargo >/dev/null 2>&1 \
  && command -v rustc >/dev/null 2>&1; then
  printf '%s\n' 'note: using the available external Rust toolchain; only Debian cargo/rustc package records are missing'
  build_dependency_flags+=(--no-check-builddeps)
else
  printf '%s\n' "$unmet_build_dependencies" >&2
  exit 1
fi

dpkg-buildpackage "${build_dependency_flags[@]}" --build=binary --no-sign --root-command=fakeroot

deb_version="$(dpkg-parsechangelog --show-field Version)"
upstream_version="$(sed -E 's/-[0-9][A-Za-z0-9.+~]*$//' <<<"$deb_version")"
architecture="$(dpkg --print-architecture)"
built_package="$(dirname "$repo_root")/gnomeengine_${deb_version}_${architecture}.deb"
source /etc/os-release
case "${VERSION_ID:-}" in
  24.04|26.04) build_release="$VERSION_ID" ;;
  *)
    printf 'error: package builds are supported only on the Ubuntu 24.04 and 26.04 validation hosts (found %s)\n' \
      "${VERSION_ID:-unknown}" >&2
    exit 2
    ;;
esac
# The filename records where the candidate was built; runtime compatibility is
# established separately by the M7 platform and package acceptance matrix.
destination="$repo_root/dist/gnomeengine_${upstream_version}_built-on-ubuntu${build_release}_${architecture}.deb"

if [[ ! -f "$built_package" ]]; then
  printf 'error: expected package was not produced: %s\n' "$built_package" >&2
  exit 1
fi

mkdir -p "$repo_root/dist"
if [[ -e "$destination" ]]; then
  existing_name="$(dpkg-deb --field "$destination" Package 2>/dev/null || true)"
  existing_version="$(dpkg-deb --field "$destination" Version 2>/dev/null || true)"
  existing_architecture="$(dpkg-deb --field "$destination" Architecture 2>/dev/null || true)"
  if [[ "$existing_name" != gnomeengine \
    || "$existing_version" != "$deb_version" \
    || "$existing_architecture" != "$architecture" ]]; then
    printf 'error: refusing to overwrite unrelated artifact: %s\n' "$destination" >&2
    exit 1
  fi
fi
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
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad gstreamer1.0-libav; do
  if ! grep -Fq "$dependency" <<<"$package_dependencies"; then
    printf 'error: package is missing required runtime dependency %s\n' "$dependency" >&2
    exit 1
  fi
done
if grep -Eq '(^|[,[:space:]])gstreamer1\.0-gtk4([,[:space:]]|$)' <<<"$package_dependencies"; then
  printf '%s\n' 'error: package must use the renderer-private GTK4 sink, not the unavailable system plugin package' >&2
  exit 1
fi
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
if [[ -d debian/gnomeengine/usr/share/gnomeengine/wallpapers ]]; then
  while IFS= read -r -d '' staged_file; do
    relative_path="${staged_file#debian/gnomeengine/}"
    if ! grep -Fq "./$relative_path" <<<"$contents"; then
      printf 'error: complete built-in wallpaper file was not packaged: %s\n' "$relative_path" >&2
      exit 1
    fi
  done < <(find debian/gnomeengine/usr/share/gnomeengine/wallpapers -type f -print0)
fi
if grep -Eq '(^|/)(target|node_modules)/|/home/|\.ts$' <<<"$contents"; then
  printf '%s\n' 'error: package contains development-only paths or TypeScript sources' >&2
  exit 1
fi

for binary in target/debian-package/release/gnomeengine target/debian-package/release/gnomeengine-renderer; do
  unresolved="$(ldd "$binary" 2>&1 | grep -F 'not found' || true)"
  if [[ -n "$unresolved" ]]; then
    printf 'error: unresolved ELF dependency in %s:\n%s\n' "$binary" "$unresolved" >&2
    exit 1
  fi
done

package_installed_size="$(dpkg-deb --field "$destination" Installed-Size)"
app_binary_size="$(stat -c '%s' target/debian-package/release/gnomeengine)"
renderer_binary_size="$(stat -c '%s' target/debian-package/release/gnomeengine-renderer)"
(
  cd "$repo_root/dist"
  sha256sum "$(basename "$destination")" > SHA256SUMS
)

printf 'built %s (%s compressed bytes)\n' "$destination" "$(stat -c '%s' "$destination")"
printf 'installed size: %s KiB; app binary: %s bytes; renderer binary: %s bytes\n' \
  "$package_installed_size" "$app_binary_size" "$renderer_binary_size"
printf 'runtime dependencies: %s\n' "$package_dependencies"
printf 'checksum: %s/dist/SHA256SUMS\n' "$repo_root"
