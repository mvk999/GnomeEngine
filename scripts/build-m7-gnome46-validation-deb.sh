#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"
dry_run=false
compare_deb=""
while (($#)); do
  case "$1" in
    --dry-run) dry_run=true ;;
    --compare) (($# >= 2)) || { echo 'error: --compare needs a .deb path' >&2; exit 2; }; compare_deb="$2"; shift ;;
    -h|--help) echo 'Usage: scripts/build-m7-gnome46-validation-deb.sh [--dry-run] [--compare normal.deb]'; exit 0 ;;
    *) printf 'error: unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done
for tool in git sha256sum dpkg dpkg-deb dpkg-parsechangelog python3 tar; do
  command -v "$tool" >/dev/null 2>&1 || { printf 'error: %s is required\n' "$tool" >&2; exit 127; }
done
[[ "$(dpkg --print-architecture)" == amd64 ]] || { echo 'error: candidate currently targets amd64 only' >&2; exit 2; }
source /etc/os-release
[[ "${ID:-}" == ubuntu && "${VERSION_ID:-}" == 24.04 ]] || {
  printf 'error: build on Ubuntu 24.04 (found %s %s)\n' "${ID:-unknown}" "${VERSION_ID:-unknown}" >&2; exit 2;
}
control_hash="$(sha256sum debian/control | awk '{print $1}')"
extension_hash="$(sha256sum extension/metadata.json | awk '{print $1}')"
metadata_unchanged() {
  [[ "$(sha256sum debian/control | awk '{print $1}')" == "$control_hash" &&
     "$(sha256sum extension/metadata.json | awk '{print $1}')" == "$extension_hash" ]]
}
stage="$(mktemp -d "${TMPDIR:-/tmp}/gnomeengine-m7test46.XXXXXXXX")"
src="$stage/source"
mkdir -p "$src"
cleanup() {
  local rc=$?
  trap - EXIT
  if metadata_unchanged; then echo 'Production metadata unchanged: PASS'; else echo 'Production metadata unchanged: FAIL' >&2; rc=1; fi
  rm -rf -- "$stage"
  exit "$rc"
}
trap cleanup EXIT
echo '==> Copy tracked and untracked source into isolated temporary staging'
git ls-files --cached --others --exclude-standard -z | tar --null -T - -cf - | tar -C "$src" -xf -

base_version="$(dpkg-parsechangelog --show-field Version)"
test_version="${base_version}~m7test46"
dpkg --compare-versions "$test_version" lt "$base_version" || { echo 'error: test version must sort below production' >&2; exit 1; }

STAGE_SOURCE="$src" TEST_VERSION="$test_version" python3 <<'PY'
import json, os, re
from pathlib import Path
root = Path(os.environ["STAGE_SOURCE"])
version = os.environ["TEST_VERSION"]
cp = root / "debian/control"
control = cp.read_text()
dep = "gnome-shell (>= 50)"
if control.count(dep) != 1: raise SystemExit("staging error: expected exactly one production dependency >= 50")
control = control.replace(dep, "gnome-shell (>= 46)", 1)
short = "Description: experimental live wallpaper manager for GNOME"
if control.count(short) != 1: raise SystemExit("staging error: package description changed")
control = control.replace(short, "Description: GnomeEngine M7 GNOME 46 validation build", 1)
old = ("external GStreamer renderer. Desktop placement uses an experimental GNOME\n"
       " Shell/Mutter bridge and requires GNOME Shell 50 or newer with the\n"
       " GnomeEngine Shell extension enabled. Integration is not yet fully\n"
       " runtime-validated on the target session.")
new = ("external GStreamer renderer. This non-release package is intended only\n"
       " to collect M7 runtime validation evidence on GNOME Shell 46.")
if old not in control: raise SystemExit("staging error: package description body changed")
cp.write_text(control.replace(old, new, 1))

mp = root / "extension/metadata.json"
metadata = json.loads(mp.read_text())
if metadata.get("uuid") != "gnomeengine@mvk999.github.io" or metadata.get("shell-version") != ["50"]:
    raise SystemExit("staging error: expected production extension metadata for Shell 50")
metadata["shell-version"] = ["46"]
mp.write_text(json.dumps(metadata, indent=2) + "\n")

ch = root / "debian/changelog"
text = ch.read_text()
first, nl, rest = text.partition("\n")
if not nl or not first.startswith("gnomeengine (") or ")" not in first:
    raise SystemExit("staging error: malformed Debian changelog")
suffix, changed = re.subn(r"\s+[^\s;]+; urgency=", " noble; urgency=", first[first.index(')')+1:], count=1)
if changed != 1:
    raise SystemExit("staging error: cannot mark validation changelog as noble")
ch.write_text(f"gnomeengine ({version}){suffix}\n{rest}")

for rel, replacements in (
    ("scripts/check-package.sh", (("JSON.stringify(['50'])", "JSON.stringify(['46'])"), ("gnome-shell (>= 50)", "gnome-shell (>= 46)"))),
    ("scripts/build-deb.sh", (("gnome-shell (>= 50)", "gnome-shell (>= 46)"),)),
):
    path = root / rel
    content = path.read_text()
    for before, after in replacements:
        if content.count(before) != 1: raise SystemExit(f"staging error: expected one {before!r} in {rel}")
        content = content.replace(before, after, 1)
    path.write_text(content)
print("Staging Debian dependency: gnome-shell (>= 46)")
print('Staging extension shell-version: ["46"]')
print(f"Staging Debian version: {version}")
print("Staging package description: GnomeEngine M7 GNOME 46 validation build")
PY

[[ "$(dpkg-parsechangelog --file "$src/debian/changelog" --show-field Version)" == "$test_version" ]]
[[ "$(sed -n '/^Package: gnomeengine$/,/^Description:/p' "$src/debian/control" | grep -Eo 'gnome-shell \(>= [^)]+\)' | head -n 1)" == 'gnome-shell (>= 46)' ]]
[[ "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["shell-version"])' "$src/extension/metadata.json")" == "['46']" ]]
if $dry_run; then
  echo '==> Canonical package metadata checks (staging copy)'
  (cd "$src" && ./scripts/check-package.sh)
  echo 'Dry run complete: metadata verified; build skipped.'
  exit 0
fi
for tool in cargo rustc dpkg-buildpackage dpkg-deb dpkg-parsechangelog dh fakeroot ldd apt-get; do
  command -v "$tool" >/dev/null 2>&1 || { printf 'BUILD TOOLCHAIN UNAVAILABLE: %s; no automatic install attempted\n' "$tool" >&2; exit 127; }
done
echo '==> Build using the canonical Debian package pipeline'
(cd "$src" && ./scripts/build-deb.sh)
upstream="$(dpkg-parsechangelog --show-field Version | sed -E 's/-[0-9][A-Za-z0-9.+~]*$//')"
canonical="$src/dist/gnomeengine_${upstream}_built-on-ubuntu24.04_amd64.deb"
[[ -f "$canonical" ]] || { printf 'error: canonical build did not produce %s\n' "$canonical" >&2; exit 1; }
name="$(dpkg-deb -f "$canonical" Package)"
version="$(dpkg-deb -f "$canonical" Version)"
arch="$(dpkg-deb -f "$canonical" Architecture)"
depends="$(dpkg-deb -f "$canonical" Depends)"
description="$(dpkg-deb -f "$canonical" Description)"
[[ "$name" == gnomeengine && "$version" == "$test_version" && "$arch" == amd64 &&
   "$description" == *'GnomeEngine M7 GNOME 46 validation build'* ]] || { echo 'error: unexpected package identity' >&2; exit 1; }
grep -Fq 'gnome-shell (>= 46)' <<<"$depends" && ! grep -Fq 'gnome-shell (>= 50)' <<<"$depends" || { echo 'error: unexpected GNOME Shell Depends' >&2; exit 1; }
for dep in cargo rustc gcc build-essential pkg-config libgtk-4-dev libadwaita-1-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev nodejs; do
  if grep -Eq "(^|[,[:space:]])${dep}([,[:space:]]|$)" <<<"$depends"; then printf 'error: build-only dependency leaked into Depends: %s\n' "$dep" >&2; exit 1; fi
done

extracted="$stage/extracted"
mkdir -p "$extracted"
dpkg-deb -x "$canonical" "$extracted"
metadata="$extracted/usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/metadata.json"
[[ -f "$metadata" && "$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["shell-version"])' "$metadata")" == "['46']" ]] || {
  echo 'error: packaged extension metadata does not declare shell-version [46]' >&2; exit 1;
}
contents="$(dpkg-deb -c "$canonical")"
for path in ./usr/bin/gnomeengine ./usr/libexec/gnomeengine/gnomeengine-renderer \
  ./usr/share/applications/io.github.mvk999.GnomeEngine.desktop \
  ./usr/share/metainfo/io.github.mvk999.GnomeEngine.metainfo.xml \
  ./usr/share/icons/hicolor/scalable/apps/io.github.mvk999.GnomeEngine.svg \
  ./usr/share/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service \
  ./usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/extension.js \
  ./usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/metadata.json \
  ./usr/share/doc/gnomeengine/THIRD_PARTY_LICENSES.md; do
  grep -Fq "$path" <<<"$contents" || { printf 'error: package is missing %s\n' "$path" >&2; exit 1; }
done
! grep -Eq '(^|/)(target|node_modules)/|/home/|\.ts$' <<<"$contents" || { echo 'error: development paths/files present in package' >&2; exit 1; }
if grep -R -a -F -l -- "$repo_root" "$extracted" >/dev/null; then
  echo 'error: package payload contains an absolute path to this repository checkout' >&2
  exit 1
fi

if [[ -z "$compare_deb" ]]; then
  for normal in "$repo_root"/dist/gnomeengine_*_built-on-ubuntu24.04_amd64.deb "$repo_root"/dist/gnomeengine_*_built-on-ubuntu26.04_amd64.deb; do
    [[ -f "$normal" ]] || continue
    if [[ "$(dpkg-deb -f "$normal" Package 2>/dev/null || true)" == gnomeengine &&
          "$(dpkg-deb -f "$normal" Version 2>/dev/null || true)" == "$base_version" ]]; then compare_deb="$normal"; break; fi
  done
fi
if [[ -n "$compare_deb" ]]; then
  [[ -r "$compare_deb" ]] || { printf 'error: unreadable comparison package %s\n' "$compare_deb" >&2; exit 1; }
  python3 - "$compare_deb" "$canonical" "$stage/compare" <<'PY'
import hashlib, subprocess, sys
from pathlib import Path
base, test, work = sys.argv[1:]
root = Path(work)
for label, package in (("base", base), ("test", test)):
    out = root / label
    out.mkdir(parents=True, exist_ok=True)
    subprocess.run(["dpkg-deb", "-x", package, str(out)], check=True)
a, b = root / "base", root / "test"
files_a = {p.relative_to(a).as_posix() for p in a.rglob("*") if p.is_file()}
files_b = {p.relative_to(b).as_posix() for p in b.rglob("*") if p.is_file()}
if files_a != files_b: raise SystemExit(f"package file list differs: missing={sorted(files_a-files_b)}, added={sorted(files_b-files_a)}")
allowed = {"usr/share/gnome-shell/extensions/gnomeengine@mvk999.github.io/metadata.json",
           "usr/share/doc/gnomeengine/changelog.Debian.gz", "usr/share/doc/gnomeengine/changelog.Debian"}
changed = [name for name in sorted(files_a) if hashlib.sha256((a/name).read_bytes()).digest() != hashlib.sha256((b/name).read_bytes()).digest()]
unexpected = [name for name in changed if name not in allowed]
if unexpected: raise SystemExit(f"unexpected package payload changes: {unexpected}")
print(f"Package file list matches; metadata/version differences: {changed or 'none'}")
PY
fi

artifact="gnomeengine_${test_version}_ubuntu24.04_amd64.deb"
mkdir -p "$repo_root/dist"
install -m 0644 "$canonical" "$repo_root/dist/$artifact"
printf '%s\n' '==> Inspect the final validation artifact'
dpkg-deb -I "$repo_root/dist/$artifact"
dpkg-deb -c "$repo_root/dist/$artifact" > "$stage/package-list.txt"
printf '%s\n' '==> Simulate APT dependency resolution for the candidate on Noble'
if ! apt_simulation="$(apt-get --simulate --no-install-recommends install "$repo_root/dist/$artifact" 2>&1)"; then
  printf '%s\n' "$apt_simulation" >&2
  echo 'error: APT could not resolve the GNOME 46 validation candidate' >&2
  exit 1
fi
printf '%s\n' "$apt_simulation"
(cd "$repo_root/dist" && sha256sum "$artifact" > "$artifact.sha256")
(
  cd "$repo_root/dist"
  sha256sum --check "$artifact.sha256"
)
sha="$(sha256sum "$repo_root/dist/$artifact" | awk '{print $1}')"
printf '\n%s\n' 'M7 GNOME 46 validation candidate ready'
printf 'Artifact: %s/dist/%s\nVersion: %s\nArchitecture: %s\nSHA256: %s\n' "$repo_root" "$artifact" "$version" "$arch" "$sha"
echo 'Production metadata unchanged: YES'
printf '\n%s\n' 'Next on Ubuntu 24.04:'
printf 'sudo apt install ./dist/%s\n' "$artifact"
echo 'Then log out/in if needed for the system extension, and run:'
printf './scripts/m7-acceptance.sh --deb ./dist/%s --video /path/to/local-test.mp4\n' "$artifact"
