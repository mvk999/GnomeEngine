# M7 Runtime Acceptance

Run this procedure from the repository checkout in each real target GNOME
session. The acceptance runner never installs packages, requests sudo, changes
production extension metadata, or downloads video. It asks for a PASS, FAIL,
or SKIP answer for each visual check; empty input is never PASS.

Beforehand, prepare a readable local H.264 MP4. A short 1920×1080, 30 FPS clip
without required audio is recommended. For GNOME 46 package tests, use the
temporary bootstrap package described below. Do not disable GNOME's version
validation globally. The runner removes common development-only
GStreamer/GSettings/library environment overrides before launching the
installed application.

Default results go to `artifacts/m7/<target>/acceptance.json` and
`acceptance.md`, with package details, GNOME/renderer logs, and diagnostics next
to them. These files are ignored by Git. To choose another result directory,
pass a target-specific path such as
`--output /path/to/results/ubuntu24.04-gnome46-wayland`; an argument ending in
`.json` selects an exact JSON path. Keep each target in a separate directory or
filename so the three runs do not overwrite one another.

## GNOME 46 bootstrap package

Production metadata remains at `gnome-shell (>= 50)` and extension
`shell-version: ["50"]` until the full M7 matrix passes, so a normal package
cannot be installed by APT on Noble. Build the candidate on an Ubuntu 24.04
build machine with the documented build toolchain:

```sh
./scripts/build-m7-gnome46-validation-deb.sh
```

The builder stages a copy under a temporary directory, changes only the staged
GNOME dependency, extension metadata, Debian test version/Noble changelog entry,
and validation description, then invokes the normal `scripts/build-deb.sh`
pipeline. It inspects the resulting `.deb` and checksum and confirms the production
metadata hashes are unchanged. The version contains `~m7test46`, the filename
identifies Ubuntu 24.04, and the description labels it as an M7 GNOME 46
validation build. It retains the `gnomeengine` package identity and installed
paths to test the real application, renderer, and extension integration.

Copy the `.deb` and its `.sha256` file to the test machine. The test machine
needs no Rust/Cargo or build dependencies; it needs the acceptance scripts
and `extension/metadata.json` (from a checkout or copied harness), runtime
dependencies, and a local test video. Install manually; the scripts never
invoke sudo:

```sh
sudo apt install ./gnomeengine_<version>~m7test46_ubuntu24.04_amd64.deb
```

Enable the system-installed extension with `gnome-extensions enable
gnomeengine@mvk999.github.io` if needed, then pass this exact `.deb` to the
acceptance runner. It records package type, filename, version, architecture,
SHA256, GNOME dependency, and packaged extension shell versions. This is test
infrastructure only: do not publish it as a release or treat it as a support
claim.

## Ubuntu 24.04 GNOME 46 Wayland

Log in to the actual Ubuntu GNOME Wayland desktop. Confirm `gnome-shell --version`
reports 46, `echo "$XDG_SESSION_TYPE"` reports `wayland`, and
`echo "$XDG_CURRENT_DESKTOP"` includes GNOME. Do not use a nested Shell for
Alt+Tab, Overview, lock, suspend, or host-session acceptance.

Install the GNOME 46 bootstrap package above and enable its system-installed
extension. Keep the package-installed app, renderer, and extension as the
components under test.

From the repository root, run:

```sh
./scripts/m7-acceptance.sh \
  --deb ./dist/gnomeengine_<version>~m7test46_ubuntu24.04_amd64.deb \
  --video /path/to/local-test.mp4
```

If the `.deb` is already installed and no candidate file is available, omit
`--deb`. The runner inspects a supplied candidate, confirms it matches the
installed version, and simulates APT resolution. It never installs it; if the
package is absent, it prints the explicit `sudo apt install ./<package>.deb`
command for the maintainer to run.

The runner confirms the real GNOME session and legacy `Meta.WaylandClient`
bridge evidence, then launches the installed GUI. Follow the prompts to import
the clip, preview and Apply it, inspect desktop stacking/input/workspaces, run
three Apply/Stop cycles, close/reopen the GUI, exercise fullscreen and pause
reason composition, lock/unlock, suspend/resume, renderer restart, and
extension reload. Battery and multiple-monitor checks are recorded as
unavailable only when that hardware is absent. The selected sink, media
metadata, process CPU/RSS samples, D-Bus status, and Shell logs are captured
where observable.

## Ubuntu 24.04 GNOME 46 X11

Log in using **Ubuntu on Xorg**. Before running the harness, verify:

```sh
gnome-shell --version
echo "$XDG_SESSION_TYPE"       # must print x11
echo "$XDG_CURRENT_DESKTOP"    # must include GNOME
```

An XWayland window in a Wayland session does not satisfy this target. Install
the GNOME 46 bootstrap package above and enable its system-installed extension.
Do not install `xprop` or `xwininfo` for the run: when already present, the
harness uses them only for diagnostics.

Run from the repository root:

```sh
./scripts/m7-acceptance.sh \
  --deb ./dist/gnomeengine_<version>~m7test46_ubuntu24.04_amd64.deb \
  --video /path/to/local-test.mp4
```

The `.deb` must already be installed and match the inspected candidate. The
runner records its EWMH desktop type, sticky/skip-taskbar/skip-pager hints,
desktop assignment, no-strut properties, focus hint, and root/renderer
geometry when those diagnostic utilities exist. It also asks all shared visual
and lifecycle questions described in the first target section. Results are
saved under `artifacts/m7/ubuntu24.04-gnome46-x11/` by default.

## Ubuntu 26.04 GNOME 50 Wayland

Use a real Ubuntu 26.04 GNOME Shell 50 Wayland session. Confirm:

```sh
gnome-shell --version
echo "$XDG_SESSION_TYPE"       # must print wayland
echo "$XDG_CURRENT_DESKTOP"    # must include GNOME
```

Enable the system-installed extension from the package; do not substitute a
GNOME 46 host or route this run through the legacy bridge.

Run:

```sh
./scripts/m7-acceptance.sh \
  --deb ./dist/gnomeengine_<version>_built-on-ubuntu24.04_amd64.deb \
  --video /path/to/local-test.mp4
```

Use the actual candidate being evaluated. The runner checks for the modern
`Meta.Window` desktop bridge and fails if evidence shows the legacy
`Meta.WaylandClient` route. It performs the same GUI, desktop, lifecycle,
repeatability, and performance prompts as the GNOME 46 runs. Results go to
`artifacts/m7/ubuntu26.04-gnome50-wayland/` by default.

## Matrix and finalization gate

After collecting the three result files, run:

```sh
./scripts/m7-report.sh
./scripts/m7-finalize.sh
```

For custom result locations, pass their common parent with `--root`, for
example `./scripts/m7-report.sh --root /path/to/results` and
`./scripts/m7-finalize.sh --root /path/to/results`.

The report labels incomplete/missing targets `MISSING`. Finalization refuses
unless each exact target has a complete PASS report with all mandatory checks
passing. A passing runtime gate only permits preparing support-metadata
changes; it does not close M7. Rebuild and validate the metadata-final package
on Ubuntu 24.04 and the same package/dependency strategy on Ubuntu 26.04 before
moving the execution plan to `docs/plans/completed/`.
