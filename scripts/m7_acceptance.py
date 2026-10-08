#!/usr/bin/env python3
"""Interactive M7 runtime acceptance runner and report gate."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tarfile
import tempfile
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
try:
    UUID = json.loads((ROOT / "extension" / "metadata.json").read_text())["uuid"]
except (OSError, KeyError, json.JSONDecodeError):
    UUID = "unavailable-extension-uuid"
BUS = "io.github.mvk999.GnomeEngine.Renderer"
OBJECT = "/io/github/mvk999/GnomeEngine/Renderer"
INTERFACE = BUS
TARGETS = {
    "ubuntu24.04-gnome46-wayland": ("24.04", 46, "wayland"),
    "ubuntu24.04-gnome46-x11": ("24.04", 46, "x11"),
    "ubuntu26.04-gnome50-wayland": ("26.04", 50, "wayland"),
}
MATRIX = [
    ("ubuntu24.04-gnome46-wayland", "Ubuntu 24.04 / GNOME 46 / Wayland"),
    ("ubuntu24.04-gnome46-x11", "Ubuntu 24.04 / GNOME 46 / X11"),
    ("ubuntu26.04-gnome50-wayland", "Ubuntu 26.04 / GNOME 50 / Wayland"),
]
REQUIRED_RUNTIME_CHECKS = {
    "environment.target_match",
    "environment.real_gnome_session",
    "environment.no_development_overrides",
    "application.package_installed",
    "application.required_files",
    "application.extension_metadata",
    "extension.enabled",
    "extension.metadata_for_current_shell",
    "media.local_file",
    "media.discovered",
    "application.gui_visible",
    "application.single_dock_entry",
    "application.import_preview_apply",
    "renderer.apply_dbus_status",
    "bridge.selected",
    "desktop.wallpaper_visible",
    "desktop.not_player_window",
    "desktop.behind_windows",
    "desktop.alt_tab",
    "desktop.overview",
    "desktop.mouse_input",
    "desktop.keyboard_focus",
    "desktop.panel_stacking",
    "desktop.notifications_stacking",
    "desktop.system_dialogs_stacking",
    "desktop.full_monitor_visual",
    "desktop.no_static_strip",
    "desktop.workspaces",
    "geometry.full_monitor_auto",
    "renderer.cycle_1_stop_visual",
    "renderer.cycle_1_stop_state",
    "renderer.cycle_1_apply_visual",
    "renderer.cycle_1_apply_state",
    "renderer.cycle_2_stop_visual",
    "renderer.cycle_2_stop_state",
    "renderer.cycle_2_apply_visual",
    "renderer.cycle_2_apply_state",
    "renderer.cycle_3_stop_visual",
    "renderer.cycle_3_stop_state",
    "renderer.cycle_3_apply_visual",
    "renderer.cycle_3_apply_state",
    "renderer.apply_stop_cycles",
    "renderer.stop_state",
    "renderer.stop_visual",
    "renderer.pause_reason_signals",
    "application.close_keeps_wallpaper",
    "application.close_keeps_wallpaper_manual",
    "application.reopen_gui_visible",
    "application.reopen_status_manual",
    "application.reopen_status",
    "lifecycle.manual_pause",
    "lifecycle.fullscreen_pause",
    "lifecycle.fullscreen_resume",
    "lifecycle.pause_reason_composition",
    "lifecycle.pause_reason_composition_manual",
    "lifecycle.stop_while_auto_paused",
    "lifecycle.stop_stays_stopped_after_condition",
    "lifecycle.apply_while_fullscreen_paused",
    "lifecycle.lock_unlock",
    "lifecycle.lock_unlock_manual",
    "lifecycle.suspend_resume",
    "lifecycle.display_power",
    "lifecycle.battery_policy",
    "desktop.monitor_topology_change",
    "lifecycle.renderer_restart",
    "lifecycle.renderer_restart_fullscreen_setup",
    "lifecycle.renderer_restart_manual",
    "lifecycle.renderer_restart_snapshot",
    "lifecycle.renderer_restart_recovery",
    "lifecycle.extension_reload",
    "lifecycle.extension_reload_manual",
    "gstreamer.sink_observed",
    "performance.playing",
    "performance.paused",
    "performance.stopped",
    "performance.paused_cheaper",
    "performance.resume_after_pause",
    "performance.restore_after_sample",
    "renderer.no_duplicate_processes",
}


def conditional_required_checks(report: dict[str, Any]) -> set[str]:
    required: set[str] = set()
    diagnostics = report.get("diagnostics", {})
    initial_power = diagnostics.get("upower_initial", {})
    if initial_power.get("battery_present"):
        required.update({"lifecycle.battery_unplugged", "lifecycle.battery_ac_restore"})
    if diagnostics.get("suspend_available") is True:
        required.add("lifecycle.suspend_visual")
    if diagnostics.get("monitors", {}).get("count") is not None and diagnostics["monitors"]["count"] > 1:
        required.add("desktop.monitor_topology_automated")
    if diagnostics.get("backlights"):
        required.add("lifecycle.display_power_manual")
    if report.get("target") == "ubuntu24.04-gnome46-x11":
        x11 = diagnostics.get("x11", {})
        if x11.get("status") != "UNAVAILABLE":
            required.add("x11.ewmh")
    return required


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def command(argv: list[str], timeout: int = 20, env: dict[str, str] | None = None,
            cwd: Path | None = None) -> dict[str, Any]:
    try:
        result = subprocess.run(
            argv,
            check=False,
            capture_output=True,
            text=True,
            timeout=timeout,
            env=env,
            cwd=cwd or ROOT,
        )
        return {"status": result.returncode, "stdout": result.stdout.strip(), "stderr": result.stderr.strip()}
    except FileNotFoundError:
        return {"status": None, "stdout": "", "stderr": f"not installed: {argv[0]}"}
    except subprocess.TimeoutExpired as error:
        stdout = error.stdout.decode(errors="replace") if isinstance(error.stdout, bytes) else (error.stdout or "")
        stderr = error.stderr.decode(errors="replace") if isinstance(error.stderr, bytes) else (error.stderr or "")
        return {"status": 124, "stdout": stdout.strip(), "stderr": (stderr + " timed out").strip()}


def key_values(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    try:
        for line in path.read_text(errors="replace").splitlines():
            if "=" in line:
                key, value = line.split("=", 1)
                result[key] = value.strip().strip('"')
    except OSError:
        pass
    return result


def proc_snapshot() -> list[dict[str, Any]]:
    uid = os.getuid()
    processes: list[dict[str, Any]] = []
    for proc in Path("/proc").glob("[0-9]*"):
        try:
            if proc.stat().st_uid != uid:
                continue
            comm = (proc / "comm").read_text().strip()
            raw = (proc / "cmdline").read_bytes().decode(errors="replace")
            args = [part for part in raw.split("\0") if part]
            exe = os.readlink(proc / "exe")
            processes.append({"pid": int(proc.name), "comm": comm, "argv": args, "exe": exe})
        except (OSError, ValueError, PermissionError):
            continue
    return processes


def binary_processes(name: str, processes: list[dict[str, Any]] | None = None) -> list[int]:
    found: list[int] = []
    for process in processes if processes is not None else proc_snapshot():
        executable_names = {Path(process["exe"]).name, process["comm"]}
        executable_names.update(Path(argument).name for argument in process["argv"][:1])
        if name in executable_names or (name == "gnomeengine-renderer" and process["comm"] == "gnomeengine-render"):
            found.append(process["pid"])
    return sorted(set(found))


def dbus_call(method: str, *args: str, timeout: int = 8) -> dict[str, Any]:
    return command(
        ["gdbus", "call", "--session", "--dest", "org.freedesktop.DBus",
         "--object-path", "/org/freedesktop/DBus", "--method", f"org.freedesktop.DBus.{method}", *args],
        timeout=timeout,
    )


def name_has_owner(name: str) -> bool | None:
    result = dbus_call("NameHasOwner", json.dumps(name))
    if result["status"] != 0:
        return None
    match = re.search(r"\b(true|false)\b", result["stdout"], re.IGNORECASE)
    return match.group(1).lower() == "true" if match else None


def session_bus_snapshot(processes: list[dict[str, Any]]) -> dict[str, Any]:
    names = dbus_call("ListNames")
    shell_owner = dbus_call("GetNameOwner", json.dumps("org.gnome.Shell"))
    bus_pid = None
    if shell_owner["status"] == 0:
        owner = re.search(r"'(:[^']+)'", shell_owner["stdout"])
        if owner:
            pid_result = dbus_call("GetConnectionUnixProcessID", json.dumps(owner.group(1)))
            pid_match = re.search(r"uint32\s+(\d+)", pid_result["stdout"])
            if pid_match:
                bus_pid = int(pid_match.group(1))
    shell_pids = [p["pid"] for p in processes if p["comm"] == "gnome-shell" or Path(p["exe"]).name == "gnome-shell"]
    return {
        "reachable": names["status"] == 0,
        "list_names_error": names["stderr"] or None,
        "gnome_shell_name_owned": shell_owner["status"] == 0,
        "gnome_shell_bus_pid": bus_pid,
        "gnome_shell_process_pids": sorted(set(shell_pids)),
        "bus_owner_matches_process": bus_pid in shell_pids if bus_pid is not None else False,
    }


def detect_environment() -> dict[str, Any]:
    os_release = key_values(Path("/etc/os-release"))
    shell = command(["gnome-shell", "--version"])
    shell_text = shell["stdout"] or shell["stderr"]
    shell_match = re.search(r"GNOME Shell\s+(\d+)(?:\.(\d+))?", shell_text)
    shell_version = shell_match.group(0).replace("GNOME Shell ", "") if shell_match else None
    shell_major = int(shell_match.group(1)) if shell_match else None
    processes = proc_snapshot()
    session_bus = session_bus_snapshot(processes)
    development_overrides = [key for key in (
        "GSETTINGS_SCHEMA_DIR", "GST_PLUGIN_PATH", "GST_PLUGIN_PATH_1_0",
        "GST_PLUGIN_SYSTEM_PATH", "GST_PLUGIN_SYSTEM_PATH_1_0", "LD_LIBRARY_PATH",
        "GI_TYPELIB_PATH", "GIO_EXTRA_MODULES", "GTK_PATH",
    ) if os.environ.get(key)]
    repository_path_entries = []
    for key in ("PATH", "XDG_DATA_DIRS"):
        for entry in os.environ.get(key, "").split(os.pathsep):
            if not entry:
                continue
            try:
                candidate = Path(entry).expanduser().resolve()
                if candidate == ROOT or ROOT in candidate.parents:
                    repository_path_entries.append({"variable": key, "path": entry})
            except OSError:
                continue
    display = os.environ.get("DISPLAY")
    wayland_display = os.environ.get("WAYLAND_DISPLAY")
    environment: dict[str, Any] = {
        "os": {
            "id": os_release.get("ID"),
            "version_id": os_release.get("VERSION_ID"),
            "pretty_name": os_release.get("PRETTY_NAME"),
        },
        "gnome": {"version_command": shell_text or None, "version": shell_version, "major": shell_major,
                  "command_status": shell["status"]},
        "session_type": os.environ.get("XDG_SESSION_TYPE"),
        "current_desktop": os.environ.get("XDG_CURRENT_DESKTOP"),
        "gtk": command(["pkg-config", "--modversion", "gtk4"])["stdout"] or None,
        "libadwaita": command(["pkg-config", "--modversion", "libadwaita-1"])["stdout"] or None,
        "gstreamer": command(["gst-launch-1.0", "--version"])["stdout"] or None,
        "architecture": command(["dpkg", "--print-architecture"])["stdout"] or None,
        "display": display or None,
        "wayland_display": wayland_display or None,
        "gnome_shell_process": {"present": bool(session_bus["gnome_shell_process_pids"]),
                                "pids": session_bus["gnome_shell_process_pids"]},
        "session_bus": session_bus,
        "session_id": os.environ.get("XDG_SESSION_ID"),
        "development_environment_overrides": development_overrides,
        "repository_path_entries": repository_path_entries,
    }
    if environment["session_id"] and shutil.which("loginctl"):
        login_session = command(["loginctl", "show-session", environment["session_id"], "-p", "Type", "-p", "Class", "-p", "State", "-p", "Remote"])
        environment["login_session"] = login_session["stdout"] or login_session["stderr"] or None
    return environment


def classify_target(environment: dict[str, Any]) -> str | None:
    os_data = environment["os"]
    desktop = (environment.get("current_desktop") or "").lower()
    for target, (version, shell_major, session) in TARGETS.items():
        if (os_data.get("id") == "ubuntu" and os_data.get("version_id") == version
                and environment["gnome"].get("major") == shell_major
                and environment.get("session_type") == session
                and "gnome" in desktop):
            return target
    return None


def real_gnome_session(environment: dict[str, Any]) -> bool:
    bus = environment["session_bus"]
    return bool(
        environment["gnome_shell_process"]["present"]
        and bus["reachable"]
        and bus["gnome_shell_name_owned"]
        and bus["bus_owner_matches_process"]
    )


class Acceptance:
    def __init__(self, output: str | None):
        self.started_epoch = int(time.time())
        self.started = now()
        self.environment = detect_environment()
        self.target = classify_target(self.environment)
        self.output_argument = output
        self.json_path, self.md_path = self.output_paths(output)
        self.report: dict[str, Any] = {
            "schema_version": 1,
            "runner_version": 1,
            "target": self.target,
            "started_at": self.started,
            "updated_at": self.started,
            "runtime_started": False,
            "sequence_complete": False,
            "overall": "INCOMPLETE",
            "environment": self.environment,
            "package": {},
            "automated": {},
            "manual": {},
            "performance": {},
            "diagnostics": {},
            "checks": {},
        }
        self.monitor_process: subprocess.Popen[str] | None = None
        self.log_start = self.started_epoch
        self.gui_pid: int | None = None
        self.video: Path | None = None
        self.app_process: subprocess.Popen[bytes] | None = None
        self.save()

    def output_paths(self, output: str | None) -> tuple[Path, Path]:
        if output:
            candidate = Path(output).expanduser().resolve()
            if candidate.suffix.lower() == ".json":
                return candidate, candidate.with_suffix(".md")
            return candidate / "acceptance.json", candidate / "acceptance.md"
        target_dir = self.target or "unsupported"
        base = ROOT / "artifacts" / "m7" / target_dir
        return base / "acceptance.json", base / "acceptance.md"

    def overall(self) -> str:
        checks = self.report["checks"]
        required = REQUIRED_RUNTIME_CHECKS | conditional_required_checks(self.report)
        missing_required = required - checks.keys()
        mandatory = [item for item in checks.values() if item.get("mandatory", False)]
        if not self.report.get("sequence_complete"):
            if self.report.get("runtime_started") and any(item.get("status") == "FAIL" for item in mandatory):
                return "FAIL"
            return "INCOMPLETE"
        if missing_required:
            return "INCOMPLETE"
        if any(item.get("status") == "FAIL" for item in mandatory):
            return "FAIL"
        if any(item.get("status") != "PASS" for item in mandatory):
            return "INCOMPLETE"
        if any(checks[name].get("mandatory", True) and checks[name].get("status") != "PASS"
               for name in required):
            return "INCOMPLETE"
        return "PASS"

    def save(self) -> None:
        self.report["updated_at"] = now()
        self.report["overall"] = self.overall()
        self.json_path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.json_path.with_suffix(self.json_path.suffix + ".tmp")
        temporary.write_text(json.dumps(self.report, indent=2, sort_keys=True) + "\n")
        temporary.replace(self.json_path)
        self.md_path.write_text(self.markdown())

    def markdown(self) -> str:
        lines = [
            "# M7 Runtime Acceptance",
            "",
            f"- Target: `{self.target or 'unsupported'}`",
            f"- Overall: **{self.report.get('overall', 'INCOMPLETE')}**",
            f"- Started: {self.report.get('started_at', self.started)}",
            f"- Updated: {self.report.get('updated_at', now())}",
            "",
            "## Environment",
            "",
            f"- OS: {self.environment['os'].get('pretty_name') or 'unavailable'}",
            f"- GNOME: {self.environment['gnome'].get('version_command') or 'unavailable'}",
            f"- Session: {self.environment.get('session_type') or 'unavailable'}",
            f"- Desktop: {self.environment.get('current_desktop') or 'unavailable'}",
            f"- GTK: {self.environment.get('gtk') or 'unavailable'}; Libadwaita: {self.environment.get('libadwaita') or 'unavailable'}",
            f"- GStreamer: {self.environment.get('gstreamer') or 'unavailable'}",
            f"- Architecture: {self.environment.get('architecture') or 'unavailable'}",
            "",
            "## Checks",
            "",
            "| Check | Status | Mandatory | Evidence |",
            "| --- | --- | --- | --- |",
        ]
        for name, item in sorted(self.report.get("checks", {}).items()):
            evidence = str(item.get("evidence", "")).replace("|", "\\|").replace("\n", " ")
            lines.append(f"| `{name}` | **{item.get('status', 'UNAVAILABLE')}** | {item.get('mandatory', False)} | {evidence} |")
        if not self.report.get("checks"):
            lines.append("| No runtime checks recorded | INCOMPLETE | — | No real target session was accepted |")
        package = self.report.get("package", {})
        lines.extend(["", "## Package", "", f"- Installed: {package.get('installed_version', 'not verified')}"])
        candidate = package.get("candidate")
        if candidate:
            lines.extend([
                f"- Type: {candidate.get('type', 'package candidate')}",
                f"- File: {candidate.get('filename')}",
                f"- Version: {candidate.get('version')}",
                f"- Architecture: {candidate.get('architecture')}",
                f"- SHA256: {candidate.get('sha256') or 'unavailable'}",
                f"- GNOME Shell dependency: {candidate.get('gnome_shell_dependency') or 'not declared'}",
                f"- Extension shell-version: {candidate.get('extension_shell_version')}",
            ])
        if package.get("dependency_simulation"):
            lines.append(f"- APT dependency simulation: {package['dependency_simulation'].get('status')} — {package['dependency_simulation'].get('summary', '')}")
        lines.extend(["", "## GStreamer and performance", ""])
        gst = self.report.get("diagnostics", {}).get("gstreamer", {})
        lines.append(f"- Sink: {gst.get('sink') or 'NOT OBSERVED'}")
        lines.append(f"- Decoder: {gst.get('decoder') or 'not observable'}")
        lines.append(f"- DMA-BUF confirmed: {gst.get('dmabuf_confirmed', False)}")
        for state, sample in sorted(self.report.get("performance", {}).items()):
            lines.append(f"- {state}: {sample}")
        lines.extend(["", "## Diagnostics", ""])
        for name, value in sorted(self.report.get("diagnostics", {}).items()):
            if name == "gstreamer":
                continue
            if isinstance(value, (dict, list)):
                text = json.dumps(value, sort_keys=True)
            else:
                text = str(value)
            if len(text) > 500:
                text = text[:497] + "..."
            escaped = text.replace("`", "\\`")
            lines.append(f"- {name}: `{escaped}`")
        lines.extend(["", "Machine logs and raw command output are stored beside this report where available.", ""])
        return "\n".join(lines)

    def check(self, name: str, status: str, evidence: Any, mandatory: bool = True) -> None:
        if status not in {"PASS", "FAIL", "SKIPPED", "UNAVAILABLE"}:
            raise ValueError(f"invalid check status {status}")
        self.report["checks"][name] = {
            "status": status,
            "mandatory": mandatory,
            "evidence": evidence,
            "updated_at": now(),
        }
        self.save()

    def print_env(self) -> None:
        env = self.environment
        print("M7 acceptance environment")
        print(f"OS: {env['os'].get('pretty_name') or 'unavailable'}")
        print(f"GNOME: {env['gnome'].get('version_command') or 'unavailable'}")
        print(f"Session: {env.get('session_type') or 'unavailable'}")
        print(f"Desktop: {env.get('current_desktop') or 'unavailable'}")
        print(f"GTK: {env.get('gtk') or 'unavailable'}")
        print(f"Libadwaita: {env.get('libadwaita') or 'unavailable'}")
        print(f"GStreamer: {env.get('gstreamer') or 'unavailable'}")
        print(f"Architecture: {env.get('architecture') or 'unavailable'}")
        print(f"DISPLAY: {env.get('display') or 'unavailable'}")
        print(f"WAYLAND_DISPLAY: {env.get('wayland_display') or 'unavailable'}")
        print(f"GNOME Shell process: {env['gnome_shell_process']['pids'] or 'not present for current user'}")
        print(f"Session D-Bus: {'reachable' if env['session_bus']['reachable'] else 'unavailable'}")
        print(f"Development environment overrides: {env['development_environment_overrides'] or 'none'}")
        print(f"Repository PATH/data entries: {env['repository_path_entries'] or 'none'}")
        print(f"Target: {self.target or 'UNSUPPORTED TEST ENVIRONMENT'}")
        print(f"Results: {self.json_path}")

    def collect_local_checks(self) -> None:
        cargo = command(["cargo", "--version"])
        rustc = command(["rustc", "--version"])
        toolchain = cargo["status"] == 0 and rustc["status"] == 0
        self.report["automated"]["build_toolchain"] = {
            "cargo": cargo["stdout"] or None,
            "rustc": rustc["stdout"] or None,
            "status": "PASS" if toolchain else "UNAVAILABLE",
        }
        if not toolchain:
            print("BUILD TOOLCHAIN UNAVAILABLE; no tools will be installed automatically.")

        logs: list[str] = []
        package_check = command([str(ROOT / "scripts" / "check-package.sh")], timeout=300)
        logs.append("$ ./scripts/check-package.sh\n" + package_check["stdout"] + "\n" + package_check["stderr"])
        self.report["automated"]["source_package_check"] = {
            "status": "PASS" if package_check["status"] == 0 else ("UNAVAILABLE" if package_check["status"] is None else "FAIL"),
            "exit_code": package_check["status"],
        }
        if toolchain:
            fast_check = command([str(ROOT / "scripts" / "check.sh")], timeout=900)
            logs.append("$ ./scripts/check.sh\n" + fast_check["stdout"] + "\n" + fast_check["stderr"])
            self.report["automated"]["workspace_check"] = {
                "status": "PASS" if fast_check["status"] == 0 else ("UNAVAILABLE" if fast_check["status"] is None else "FAIL"),
                "exit_code": fast_check["status"],
            }
        else:
            self.report["automated"]["workspace_check"] = {"status": "UNAVAILABLE", "reason": "Cargo or rustc missing; package runtime remains possible"}
        self.json_path.parent.mkdir(parents=True, exist_ok=True)
        (self.json_path.parent / "local-checks.log").write_text("\n\n".join(logs) + "\n")
        self.save()

    def audit_deb(self, deb_path: Path) -> None:
        package: dict[str, Any] = self.report["package"]
        package["candidate_path"] = str(deb_path)
        package["candidate_readable"] = deb_path.is_file() and os.access(deb_path, os.R_OK)
        if not package["candidate_readable"]:
            package["candidate_audit"] = {"status": "FAIL", "reason": "--deb is not a readable regular file"}
            self.check("package.candidate_audit", "FAIL", "--deb is not a readable regular file", mandatory=False)
            return
        info = command(["dpkg-deb", "-I", str(deb_path)], timeout=60)
        listing = command(["dpkg-deb", "-c", str(deb_path)], timeout=60)
        self.json_path.parent.mkdir(parents=True, exist_ok=True)
        (self.json_path.parent / "package-info.txt").write_text(info["stdout"] + "\n" + info["stderr"] + "\n")
        (self.json_path.parent / "package-list.txt").write_text(listing["stdout"] + "\n" + listing["stderr"] + "\n")
        fields = command(["dpkg-deb", "-f", str(deb_path), "Package", "Version", "Architecture"], timeout=60)
        depends_field = command(["dpkg-deb", "-f", str(deb_path), "Depends"], timeout=60)
        lines = fields["stdout"].splitlines()
        name, version, architecture = (lines + [""] * 3)[:3]
        depends = depends_field["stdout"].replace("\n", " ")
        sha_result = command(["sha256sum", str(deb_path)], timeout=60)
        package_sha256 = sha_result["stdout"].split()[0] if sha_result["status"] == 0 and sha_result["stdout"].split() else None
        extension_shell_versions: list[str] | None = None
        with tempfile.TemporaryDirectory(prefix="gnomeengine-m7-deb-") as extract_dir:
            extracted = command(["dpkg-deb", "-x", str(deb_path), extract_dir], timeout=60)
            packaged_metadata = Path(extract_dir) / "usr/share/gnome-shell/extensions" / UUID / "metadata.json"
            if extracted["status"] == 0:
                try:
                    metadata_value = json.loads(packaged_metadata.read_text())
                    value = metadata_value.get("shell-version")
                    if isinstance(value, list) and all(isinstance(item, str) for item in value):
                        extension_shell_versions = value
                except (OSError, json.JSONDecodeError):
                    pass
        paths = [line.rsplit(None, 1)[-1].removeprefix("./") for line in listing["stdout"].splitlines() if line.strip()]
        expected = [
            "usr/bin/gnomeengine",
            "usr/libexec/gnomeengine/gnomeengine-renderer",
            "usr/share/applications/io.github.mvk999.GnomeEngine.desktop",
            "usr/share/metainfo/io.github.mvk999.GnomeEngine.metainfo.xml",
            "usr/share/icons/hicolor/scalable/apps/io.github.mvk999.GnomeEngine.svg",
            f"usr/share/gnome-shell/extensions/{UUID}/extension.js",
            f"usr/share/gnome-shell/extensions/{UUID}/metadata.json",
            "usr/share/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service",
            "usr/share/doc/gnomeengine/copyright",
            "usr/share/doc/gnomeengine/THIRD_PARTY_LICENSES.md",
        ]
        missing = [path for path in expected if path not in paths]
        forbidden_paths = [path for path in paths if re.search(r"(^|/)(target|node_modules)(/|$)", path)]
        no_system_gtk_sink = not re.search(r"(^|[,[:space:]])gstreamer1\.0-gtk4([,[:space:]]|$)", depends)
        shell_dependency = re.search(r"gnome-shell\s*\(>=\s*([^\)]+)\)", depends)
        tested_major = TARGETS[self.target][1] if self.target else None
        extension_supports_target = tested_major is not None and str(tested_major) in (extension_shell_versions or [])
        no_repo_path = self.audit_embedded_repo_paths(deb_path)
        identity_ok = name == "gnomeengine" and architecture == (self.environment.get("architecture") or "")
        package["candidate"] = {
            "type": "M7 GNOME46 validation candidate" if "~m7test46" in version else "regular package candidate",
            "filename": deb_path.name,
            "name": name,
            "version": version,
            "architecture": architecture,
            "sha256": package_sha256,
            "depends": depends,
            "gnome_shell_dependency": shell_dependency.group(0) if shell_dependency else None,
            "gnome_shell_minimum": shell_dependency.group(1) if shell_dependency else None,
            "extension_shell_version": extension_shell_versions,
            "extension_supports_tested_shell": extension_supports_target,
            "expected_paths_missing": missing,
            "development_paths": forbidden_paths,
            "contains_gstreamer1_0_gtk4_dependency": not no_system_gtk_sink,
            "embedded_repository_path_found": not no_repo_path,
            "identity_matches_host": identity_ok,
        }
        audit_pass = bool(
            info["status"] == 0 and listing["status"] == 0 and fields["status"] == 0 and depends_field["status"] == 0
            and identity_ok and not missing and not forbidden_paths and no_system_gtk_sink and no_repo_path
            and extension_supports_target and package_sha256 is not None
        )
        package["candidate_audit"] = {"status": "PASS" if audit_pass else "FAIL", "missing": missing,
                                      "forbidden_paths": forbidden_paths, "no_repo_absolute_paths": no_repo_path}
        self.check("package.candidate_audit", "PASS" if audit_pass else "FAIL", package["candidate"], mandatory=False)
        if shell_dependency:
            try:
                minimum_major = int(shell_dependency.group(1).strip().split(".", 1)[0])
                tested_major = TARGETS[self.target][1] if self.target else None
                dependency_ok = tested_major is not None and minimum_major <= tested_major
            except ValueError:
                minimum_major = None
                tested_major = TARGETS[self.target][1] if self.target else None
                dependency_ok = False
            self.check("package.gnome_dependency", "PASS" if dependency_ok else "FAIL",
                       {"declared": shell_dependency.group(0), "tested_shell_major": tested_major,
                        "minimum_major": minimum_major}, mandatory=False)
            if not dependency_ok and tested_major is not None:
                package["candidate_gnome_dependency_note"] = (
                    f"The candidate requires GNOME Shell {minimum_major or shell_dependency.group(1)}, "
                    f"which does not include tested Shell {tested_major}; APT resolution is reported separately."
                )
        else:
            self.check("package.gnome_dependency", "FAIL", "No explicit gnome-shell minimum was declared", mandatory=False)
        simulation = command(["apt-get", "--simulate", "--no-install-recommends", "install", str(deb_path)], timeout=180)
        package["dependency_simulation"] = {
            "status": "PASS" if simulation["status"] == 0 else ("UNAVAILABLE" if simulation["status"] is None else "FAIL"),
            "exit_code": simulation["status"],
            "summary": (simulation["stdout"] + "\n" + simulation["stderr"])[-4000:],
        }
        self.check("package.dependency_resolution", package["dependency_simulation"]["status"],
                   package["dependency_simulation"]["summary"], mandatory=False)
        self.save()

    def audit_embedded_repo_paths(self, deb_path: Path) -> bool:
        """Return True when no text payload contains this checkout's absolute path."""
        try:
            process = subprocess.Popen(["dpkg-deb", "--fsys-tarfile", str(deb_path)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        except OSError:
            return False
        needle = str(ROOT).encode()
        found = False
        try:
            assert process.stdout is not None
            with tarfile.open(fileobj=process.stdout, mode="r|") as archive:
                for member in archive:
                    if not member.isfile():
                        continue
                    stream = archive.extractfile(member)
                    if stream is None:
                        continue
                    carry = b""
                    while True:
                        data = stream.read(128 * 1024)
                        if not data:
                            break
                        if needle in carry + data:
                            found = True
                        carry = (carry + data)[-(len(needle) - 1):]
        except (tarfile.TarError, OSError):
            found = True
        finally:
            if process.stdout:
                process.stdout.close()
            process.wait(timeout=60)
        return not found

    def installed_package(self) -> bool:
        query = command(["dpkg-query", "-W", "-f=${Version}\t${Architecture}", "gnomeengine"])
        if query["status"] != 0:
            self.report["package"]["installed_version"] = None
            self.check("application.package_installed", "FAIL", "gnomeengine is not installed")
            return False
        version, _, architecture = query["stdout"].partition("\t")
        package = self.report["package"]
        package["installed_version"] = version
        package["installed_architecture"] = architecture
        expected = {
            "/usr/bin/gnomeengine": Path("/usr/bin/gnomeengine"),
            "/usr/libexec/gnomeengine/gnomeengine-renderer": Path("/usr/libexec/gnomeengine/gnomeengine-renderer"),
            f"/usr/share/gnome-shell/extensions/{UUID}/extension.js": Path(f"/usr/share/gnome-shell/extensions/{UUID}/extension.js"),
            f"/usr/share/gnome-shell/extensions/{UUID}/metadata.json": Path(f"/usr/share/gnome-shell/extensions/{UUID}/metadata.json"),
            "/usr/share/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service": Path("/usr/share/dbus-1/services/io.github.mvk999.GnomeEngine.Renderer.service"),
            "/usr/share/applications/io.github.mvk999.GnomeEngine.desktop": Path("/usr/share/applications/io.github.mvk999.GnomeEngine.desktop"),
            "/usr/share/icons/hicolor/scalable/apps/io.github.mvk999.GnomeEngine.svg": Path("/usr/share/icons/hicolor/scalable/apps/io.github.mvk999.GnomeEngine.svg"),
            "/usr/share/metainfo/io.github.mvk999.GnomeEngine.metainfo.xml": Path("/usr/share/metainfo/io.github.mvk999.GnomeEngine.metainfo.xml"),
        }
        missing = [str(path) for path in expected.values() if not path.is_file()]
        package["installed_paths_missing"] = missing
        installed_extension_metadata_path = Path(f"/usr/share/gnome-shell/extensions/{UUID}/metadata.json")
        try:
            installed_extension_metadata = json.loads(installed_extension_metadata_path.read_text())
        except (OSError, json.JSONDecodeError):
            installed_extension_metadata = {}
        package["extension_metadata"] = {
            "uuid": installed_extension_metadata.get("uuid"),
            "shell_version": installed_extension_metadata.get("shell-version"),
            "matches_repository_uuid": installed_extension_metadata.get("uuid") == UUID,
        }
        app_path = shutil.which("gnomeengine")
        package["installed_application_path"] = app_path
        owned = command(["dpkg-query", "-S", app_path]) if app_path else {"status": 1, "stdout": "", "stderr": "command unavailable"}
        package["application_owned_by_package"] = owned["status"] == 0 and "gnomeengine:" in owned["stdout"]
        schema = command(["gsettings", "list-schemas"])
        package["gsettings"] = {
            "host_shell_schema_available": "org.gnome.shell" in schema["stdout"].splitlines(),
            "application_schema_expected": False,
            "application_storage": "renderer and GUI store lifecycle preferences in XDG config lifecycle.ini; the extension uses the host-provided org.gnome.shell schema",
        }
        package_ok = not missing and package["application_owned_by_package"] and architecture == (self.environment.get("architecture") or "")
        self.check("application.package_installed", "PASS" if package_ok else "FAIL",
                   {"version": version, "architecture": architecture, "missing": missing,
                    "app_path": app_path, "package_owned": package["application_owned_by_package"]})
        self.check("application.required_files", "PASS" if not missing else "FAIL", {"missing": missing})
        metadata_ok = package["extension_metadata"]["matches_repository_uuid"]
        self.check("application.extension_metadata", "PASS" if metadata_ok else "FAIL",
                   package["extension_metadata"], mandatory=True)
        self.check("application.gsettings_compatibility", "PASS" if package["gsettings"]["host_shell_schema_available"] else "UNAVAILABLE",
                   package["gsettings"], mandatory=False)
        candidate = package.get("candidate")
        if candidate:
            same = candidate.get("version") == version and candidate.get("architecture") == architecture
            package["candidate_matches_installed"] = same
            self.check("package.candidate_matches_installed", "PASS" if same else "FAIL",
                       {"candidate": candidate.get("version"), "installed": version}, mandatory=False)
            if not same:
                package["runtime_candidate_mismatch"] = True
        self.save()
        return package_ok

    def video_precheck(self, video_arg: str | None) -> bool:
        value = video_arg
        while not value:
            try:
                value = input("Local test video path (H.264 MP4, 1920x1080/30 FPS recommended): ").strip()
            except EOFError:
                value = ""
                break
            if not value:
                print("A readable local video is required. Supply --video /path/to/test.mp4.")
        if not value:
            self.check("media.local_file", "UNAVAILABLE", "No local test video was provided")
            return False
        path = Path(value).expanduser().resolve()
        valid = path.is_file() and os.access(path, os.R_OK)
        self.report["diagnostics"]["video_path"] = str(path)
        self.video = path if valid else None
        self.check("media.local_file", "PASS" if valid else "FAIL", str(path) if valid else "Not a readable regular local file")
        if not valid:
            return False
        discoverer = command(["gst-discoverer-1.0", str(path)], timeout=90)
        self.json_path.parent.mkdir(parents=True, exist_ok=True)
        (self.json_path.parent / "gst-discoverer.txt").write_text(discoverer["stdout"] + "\n" + discoverer["stderr"] + "\n")
        info = discoverer["stdout"]
        codec = re.search(r"video #\d+:\s*([^\n]+)", info, re.IGNORECASE)
        width = re.search(r"Width:\s*(\d+)", info, re.IGNORECASE)
        height = re.search(r"Height:\s*(\d+)", info, re.IGNORECASE)
        fps = re.search(r"Frame rate:\s*([^\n]+)", info, re.IGNORECASE)
        self.report["diagnostics"]["media"] = {
            "codec": codec.group(1).strip() if codec else None,
            "width": int(width.group(1)) if width else None,
            "height": int(height.group(1)) if height else None,
            "fps": fps.group(1).strip() if fps else None,
            "discoverer_status": discoverer["status"],
        }
        success = discoverer["status"] == 0 and bool(codec)
        status = "PASS" if success else ("UNAVAILABLE" if discoverer["status"] is None else "FAIL")
        self.check("media.discovered", status, self.report["diagnostics"]["media"])
        self.save()
        return success

    def extension_info(self) -> dict[str, Any]:
        info = command(["gnome-extensions", "info", UUID])
        text = info["stdout"] + "\n" + info["stderr"]
        state = re.search(r"(?:state|status)\s*:\s*([A-Za-z_-]+)", text, re.IGNORECASE)
        path = re.search(r"(?:path)\s*:\s*(.+)", text, re.IGNORECASE)
        extension_path = Path(path.group(1).strip()) if path else None
        metadata_path = extension_path / "metadata.json" if extension_path and extension_path.is_dir() else extension_path
        try:
            metadata = json.loads(metadata_path.read_text()) if metadata_path else {}
        except (OSError, json.JSONDecodeError):
            metadata = {}
        details = {"exit_code": info["status"], "output": text.strip() or None,
                   "state": state.group(1).upper() if state else None,
                   "path": str(extension_path) if extension_path else None,
                   "metadata_shell_versions": metadata.get("shell-version"),
                   "metadata_uuid": metadata.get("uuid")}
        self.report["diagnostics"]["extension_info"] = details
        self.save()
        return details

    def ensure_extension_enabled(self) -> bool:
        details = self.extension_info()
        shell_major = self.environment["gnome"].get("major")
        version_supported = str(shell_major) in (details.get("metadata_shell_versions") or [])
        if details["state"] == "ENABLED" and details.get("metadata_uuid") == UUID and version_supported:
            self.check("extension.enabled", "PASS", details)
            self.check("extension.metadata_for_current_shell", "PASS", details)
            return True
        print(f"Extension state: {details['state'] or 'unavailable'}.")
        print(f"Enable the packaged extension with: gnome-extensions enable {UUID}")
        if self.target and self.target.startswith("ubuntu24.04"):
            print("If GNOME 46 rejects production metadata, use the temporary test-copy workflow in docs/engineering/manual-testing.md.")
        try:
            input("After enabling/installing the test copy, press Enter to recheck (this does not record PASS): ")
        except EOFError:
            pass
        details = self.extension_info()
        version_supported = str(shell_major) in (details.get("metadata_shell_versions") or [])
        enabled = details["state"] == "ENABLED" and details.get("metadata_uuid") == UUID and version_supported
        self.check("extension.enabled", "PASS" if enabled else "FAIL", details)
        self.check("extension.metadata_for_current_shell", "PASS" if version_supported else "FAIL",
                   {"running_shell_major": shell_major, "extension_metadata": details}, mandatory=True)
        return enabled

    def prompt(self, key: str, question: str, instructions: str = "", mandatory: bool = True) -> str:
        if instructions:
            print(instructions)
        print(f"[MANUAL] {question}")
        print("  [p] PASS   [f] FAIL   [s] SKIP")
        while True:
            try:
                answer = input("Result (p/f/s): ").strip().lower()
            except EOFError:
                answer = "s"
            if answer in {"p", "pass"}:
                status = "PASS"
                break
            if answer in {"f", "fail"}:
                status = "FAIL"
                break
            if answer in {"s", "skip"}:
                status = "SKIPPED"
                break
            print("Enter p, f, or s. Empty input never means PASS.")
        self.report["manual"][key] = {"status": status, "mandatory": mandatory, "question": question, "instructions": instructions}
        self.check(key, status, {"answer": answer, "question": question}, mandatory=mandatory)
        return status

    def ask_continue(self, message: str) -> None:
        try:
            input(message + "\nPress Enter to continue; the next result still requires its own p/f/s answer. ")
        except EOFError:
            pass

    def renderer_owner(self) -> bool | None:
        return name_has_owner(BUS)

    def get_status(self) -> dict[str, Any] | None:
        owner = self.renderer_owner()
        if owner is not True:
            return None
        result = command(["gdbus", "call", "--session", "--dest", BUS, "--object-path", OBJECT,
                          "--method", f"{INTERFACE}.GetStatus"], timeout=8)
        if result["status"] != 0:
            return None
        raw = result["stdout"]
        state = re.search(r"'state'\s*:\s*<['\"]([^'\"]+)['\"]>", raw)
        current = re.search(r"'currentVideo'\s*:\s*<['\"](.*?)['\"]>", raw)
        reasons_field = re.search(r"'pauseReasons'\s*:\s*<\[([^\]]*)\]>", raw)
        reasons = re.findall(r"['\"]([^'\"]+)['\"]", reasons_field.group(1)) if reasons_field else []

        def boolean(field: str) -> bool | None:
            match = re.search(rf"'{re.escape(field)}'\s*:\s*<(true|false)>", raw, re.IGNORECASE)
            return match.group(1).lower() == "true" if match else None

        status = {
            "state": state.group(1) if state else None,
            "current_video": current.group(1) if current else None,
            "pause_reasons": reasons,
            "wallpaper_active": boolean("wallpaperActive"),
            "desktop_integration_ready": boolean("desktopIntegrationReady"),
            "pause_on_battery": boolean("pauseOnBattery"),
            "pause_on_low_battery_only": boolean("pauseOnLowBatteryOnly"),
            "last_error": (re.search(r"'lastError'\s*:\s*<['\"](.*?)['\"]>", raw).group(1)
                           if re.search(r"'lastError'\s*:\s*<['\"](.*?)['\"]>", raw) else None),
            "raw": raw,
        }
        self.report["diagnostics"].setdefault("renderer_status_history", []).append({"at": now(), **status})
        if len(self.report["diagnostics"]["renderer_status_history"]) > 120:
            self.report["diagnostics"]["renderer_status_history"] = self.report["diagnostics"]["renderer_status_history"][-120:]
        return status

    def renderer_call(self, method: str, *arguments: str) -> tuple[bool, str]:
        if self.renderer_owner() is not True:
            return False, "renderer D-Bus name is not owned; not invoking an activation-prone method"
        result = command(["gdbus", "call", "--session", "--dest", BUS, "--object-path", OBJECT,
                          "--method", f"{INTERFACE}.{method}", *arguments], timeout=20)
        return result["status"] == 0, result["stdout"] or result["stderr"]

    def wait_status(self, predicate, timeout: int = 25) -> dict[str, Any] | None:
        end = time.monotonic() + timeout
        last = None
        while time.monotonic() < end:
            last = self.get_status()
            if last and predicate(last):
                return last
            time.sleep(1)
        return last

    def process_counts(self) -> dict[str, list[int]]:
        processes = proc_snapshot()
        return {name: binary_processes(name, processes) for name in ("gnomeengine", "gnomeengine-renderer")}

    def start_signal_monitor(self) -> None:
        path = self.json_path.parent / "gdbus-monitor.log"
        stream = path.open("w")
        try:
            self.monitor_process = subprocess.Popen(
                ["gdbus", "monitor", "--session", "--dest", BUS, "--object-path", OBJECT],
                stdout=stream,
                stderr=subprocess.STDOUT,
                text=True,
            )
            # Retain the file handle through the child lifetime.
            self.report["diagnostics"]["signal_monitor_log"] = str(path)
            self.report["diagnostics"]["signal_monitor_available"] = True
            self._monitor_stream = stream
        except OSError as error:
            stream.close()
            self.report["diagnostics"]["signal_monitor_available"] = False
            self.report["diagnostics"]["signal_monitor_error"] = str(error)
        self.save()

    def stop_signal_monitor(self) -> None:
        if self.monitor_process:
            self.monitor_process.terminate()
            try:
                self.monitor_process.wait(timeout=4)
            except subprocess.TimeoutExpired:
                self.monitor_process.kill()
                self.monitor_process.wait(timeout=4)
            try:
                self._monitor_stream.close()
            except (AttributeError, OSError):
                pass
            self.monitor_process = None

    def shell_logs(self) -> str:
        since = f"@{self.log_start}"
        attempts = [
            ["journalctl", "-b", "--no-pager", "-o", "short-iso-precise", f"--since={since}", f"_UID={os.getuid()}", "_COMM=gnome-shell"],
            ["journalctl", "--user", "-b", "--no-pager", "-o", "short-iso-precise", f"--since={since}", "_COMM=gnome-shell"],
        ]
        collected = []
        for argv in attempts:
            result = command(argv, timeout=30)
            if result["status"] == 0 and result["stdout"]:
                collected.append(result["stdout"])
        return "\n".join(dict.fromkeys(line for part in collected for line in part.splitlines()))

    def renderer_logs(self) -> str:
        since = f"@{self.log_start}"
        attempts = [
            ["journalctl", "-b", "--no-pager", "-o", "short-iso-precise", f"--since={since}",
             f"_UID={os.getuid()}", "_EXE=/usr/libexec/gnomeengine/gnomeengine-renderer"],
            ["journalctl", "--user", "-b", "--no-pager", "-o", "short-iso-precise", f"--since={since}",
             "_COMM=gnomeengine-ren"],
        ]
        relevant = []
        for argv in attempts:
            result = command(argv, timeout=30)
            if result["status"] == 0:
                relevant.extend(line for line in result["stdout"].splitlines()
                                if any(word in line.lower() for word in ("gnomeengine", "gtk4paintablesink", "dma-buf", "decoder")))
        return "\n".join(dict.fromkeys(relevant))

    def save_session_logs(self) -> tuple[str, str]:
        shell = self.shell_logs()
        renderer = self.renderer_logs()
        self.json_path.parent.mkdir(parents=True, exist_ok=True)
        (self.json_path.parent / "gnome-shell.log").write_text(shell + "\n")
        (self.json_path.parent / "renderer.log").write_text(renderer + "\n")
        self.report["diagnostics"]["shell_log_lines"] = len(shell.splitlines())
        self.report["diagnostics"]["renderer_log_lines"] = len(renderer.splitlines())
        self.report["diagnostics"]["shell_logs_available"] = bool(shell)
        self.report["diagnostics"]["renderer_logs_available"] = bool(renderer)
        self.save()
        return shell, renderer

    def x11_diagnostics(self) -> None:
        if self.target != "ubuntu24.04-gnome46-x11":
            return
        xprop = shutil.which("xprop")
        xwininfo = shutil.which("xwininfo")
        if not xprop or not xwininfo:
            missing = [tool for tool, path in (("xprop", xprop), ("xwininfo", xwininfo)) if not path]
            self.report["diagnostics"]["x11"] = {"status": "UNAVAILABLE", "missing_tools": missing}
            self.check("x11.ewmh", "UNAVAILABLE", f"Optional diagnostics missing: {', '.join(missing)}", mandatory=False)
            return
        tree = command([xwininfo, "-root", "-tree"])
        match = re.search(r"^\s*(0x[0-9a-fA-F]+)\s+\"GnomeEngine Wallpaper Surface\"", tree["stdout"], re.MULTILINE)
        if not match:
            self.report["diagnostics"]["x11"] = {"status": "FAIL", "reason": "renderer window not found in X11 root tree"}
            self.check("x11.ewmh", "FAIL", self.report["diagnostics"]["x11"], mandatory=True)
            return
        window_id = match.group(1)
        properties = command([xprop, "-id", window_id, "_NET_WM_WINDOW_TYPE", "_NET_WM_STATE", "_NET_WM_DESKTOP", "WM_HINTS"])
        strut = command([xprop, "-id", window_id, "_NET_WM_STRUT"])
        partial = command([xprop, "-id", window_id, "_NET_WM_STRUT_PARTIAL"])
        geometry = command([xwininfo, "-id", window_id])
        root = command([xwininfo, "-root"])
        output = "\n".join([properties["stdout"], strut["stdout"], partial["stdout"], geometry["stdout"], root["stdout"]])
        self.json_path.parent.mkdir(parents=True, exist_ok=True)
        (self.json_path.parent / "x11-window-diagnostics.txt").write_text(output + "\n")
        state_line = properties["stdout"]
        window_geometry = re.search(r"-geometry\s+(\d+x\d+[+-]\d+[+-]\d+)", geometry["stdout"])
        root_geometry = re.search(r"-geometry\s+(\d+x\d+[+-]\d+[+-]\d+)", root["stdout"])
        checks = {
            "desktop_type": "_NET_WM_WINDOW_TYPE_DESKTOP" in state_line,
            "sticky": "_NET_WM_STATE_STICKY" in state_line or "4294967295" in state_line or "0xffffffff" in state_line.lower(),
            "skip_taskbar": "_NET_WM_STATE_SKIP_TASKBAR" in state_line,
            "skip_pager": "_NET_WM_STATE_SKIP_PAGER" in state_line,
            "non_focusable": bool(re.search(
                r"(?:input:\s*False|Client accepts input or input focus:\s*False)", state_line, re.IGNORECASE)),
            "no_strut": "_NET_WM_STRUT" not in strut["stdout"] or any(marker in strut["stdout"].lower() for marker in ("not found", "no such atom", "no atom")),
            "no_partial_strut": "_NET_WM_STRUT_PARTIAL" not in partial["stdout"] or any(marker in partial["stdout"].lower() for marker in ("not found", "no such atom", "no atom")),
            "full_root_geometry": bool(window_geometry and root_geometry and window_geometry.group(1) == root_geometry.group(1)),
        }
        diagnostic = {
            "window_id": window_id,
            "properties": properties["stdout"],
            "strut": strut["stdout"],
            "strut_partial": partial["stdout"],
            "window_geometry": window_geometry.group(1) if window_geometry else None,
            "root_geometry": root_geometry.group(1) if root_geometry else None,
            "checks": checks,
            "raw_log": "x11-window-diagnostics.txt",
        }
        self.report["diagnostics"]["x11"] = diagnostic
        passed = all(checks.values())
        self.check("x11.ewmh", "PASS" if passed else "FAIL", diagnostic, mandatory=True)
        print("X11 EWMH diagnostic:")
        for key, value in checks.items():
            print(f"  {key}: {'PASS' if value else 'FAIL'}")

    def monitor_snapshot(self) -> dict[str, Any]:
        result: dict[str, Any] = {"count": None, "source": None, "raw": None}
        xrandr = shutil.which("xrandr")
        if self.environment.get("session_type") == "x11" and xrandr:
            monitors = command([xrandr, "--listactivemonitors"])
            match = re.search(r"Monitors:\s*(\d+)", monitors["stdout"])
            if match:
                result.update({"count": int(match.group(1)), "source": "xrandr --listactivemonitors", "raw": monitors["stdout"]})
                return result
        display_config = command([
            "gdbus", "call", "--session", "--dest", "org.gnome.Mutter.DisplayConfig",
            "--object-path", "/org/gnome/Mutter/DisplayConfig",
            "--method", "org.gnome.Mutter.DisplayConfig.GetCurrentState",
        ])
        if display_config["status"] == 0:
            raw = display_config["stdout"]
            result.update({"source": "Mutter DisplayConfig.GetCurrentState", "raw": raw})
            # The signature starts with a serial and then a top-level monitor array.
            match = re.match(r"^\(uint32\s+\d+,\s*(\[)", raw)
            if match:
                start = match.start(1)
                depth = 0
                end = None
                for position in range(start, len(raw)):
                    if raw[position] == "[":
                        depth += 1
                    elif raw[position] == "]":
                        depth -= 1
                        if depth == 0:
                            end = position
                            break
                if end is not None:
                    body = raw[start + 1:end]
                    depth = 0
                    quoted = False
                    escaped = False
                    count = 0
                    element_started = False
                    for char in body:
                        if escaped:
                            escaped = False
                            continue
                        if char == "\\" and quoted:
                            escaped = True
                            continue
                        if char == "'":
                            quoted = not quoted
                            continue
                        if quoted:
                            continue
                        if char in "([{":
                            depth += 1
                            element_started = True
                        elif char in ")]}" :
                            depth -= 1
                        elif char == "," and depth == 0:
                            if element_started:
                                count += 1
                                element_started = False
                    if element_started:
                        count += 1
                    result["count"] = count
        return result

    def geometry_from_logs(self, shell_log: str) -> dict[str, Any] | None:
        pattern = re.compile(r"wallpaper geometry monitor=([^ ]+) work-area=([^ ]+) renderer=([^\s]+)")
        matches = pattern.findall(shell_log)
        if not matches:
            self.check("geometry.full_monitor_auto", "UNAVAILABLE", "No wallpaper geometry line was found in GNOME Shell logs", mandatory=False)
            return None
        monitor, work_area, renderer = matches[-1]
        equal = monitor == renderer
        geometry = {"monitor": monitor, "work_area_diagnostic_only": work_area, "renderer": renderer, "matches_full_monitor": equal}
        self.report["diagnostics"]["geometry"] = geometry
        self.check("geometry.full_monitor_auto", "PASS" if equal else "FAIL", geometry, mandatory=True)
        return geometry

    def x11_active_monitor_count(self) -> int | None:
        snapshot = self.monitor_snapshot()
        self.report["diagnostics"]["monitors"] = snapshot
        self.save()
        return snapshot.get("count")

    def upower_snapshot(self) -> dict[str, Any]:
        batteries = []
        supplies = Path("/sys/class/power_supply")
        if supplies.exists():
            for supply in supplies.iterdir():
                try:
                    kind = (supply / "type").read_text().strip()
                except OSError:
                    continue
                if kind.lower() == "battery":
                    batteries.append(supply.name)
        detail = command(["upower", "-d"], timeout=15)
        on_battery_result = command([
            "gdbus", "call", "--system", "--dest", "org.freedesktop.UPower",
            "--object-path", "/org/freedesktop/UPower",
            "--method", "org.freedesktop.DBus.Properties.Get", "org.freedesktop.UPower", "OnBattery",
        ])
        on_battery_match = re.search(r"<(true|false)>", on_battery_result["stdout"], re.IGNORECASE)
        percentage_result = command([
            "gdbus", "call", "--system", "--dest", "org.freedesktop.UPower",
            "--object-path", "/org/freedesktop/UPower/devices/DisplayDevice",
            "--method", "org.freedesktop.DBus.Properties.Get", "org.freedesktop.UPower.Device", "Percentage",
        ])
        percentage_match = re.search(r"<([0-9]+(?:\.[0-9]+)?)>", percentage_result["stdout"])
        if not batteries and detail["status"] == 0:
            batteries = sorted(set(re.findall(
                r"/org/freedesktop/UPower/devices/(battery_[^\s]+)", detail["stdout"]
            )))
        return {
            "battery_present": bool(batteries),
            "battery_devices": batteries,
            "on_battery": (on_battery_match.group(1).lower() == "true") if on_battery_match else None,
            "percentage": float(percentage_match.group(1)) if percentage_match else None,
            "upower_available": detail["status"] == 0 and on_battery_result["status"] == 0,
            "raw": detail["stdout"][-6000:],
        }

    def battery_test(self) -> None:
        before = self.upower_snapshot()
        self.report["diagnostics"]["upower_initial"] = before
        if not before["battery_present"]:
            self.check("lifecycle.battery_policy", "UNAVAILABLE", "No battery is present; AC/battery transition is hardware-specific", mandatory=False)
            self.check("lifecycle.low_battery_threshold", "UNAVAILABLE", "No battery is present", mandatory=False)
            return
        if not before["upower_available"] or before["on_battery"] is None:
            self.check("lifecycle.battery_policy", "UNAVAILABLE", "UPower OnBattery state could not be read", mandatory=True)
            return
        if before["on_battery"]:
            self.ask_continue("UPower currently reports OnBattery=yes. Connect AC, return here, and continue only after UPower reports OnBattery=no.")
            on_ac = self.upower_snapshot()
            self.report["diagnostics"]["upower_ac_baseline"] = on_ac
            if on_ac.get("on_battery") is not False:
                self.check("lifecycle.battery_policy", "FAIL", {"reason": "Could not establish an AC baseline", "upower": on_ac})
                return
        current = self.get_status()
        print("Battery policy: pause-on-battery=" + str((current or {}).get("pause_on_battery")) +
              "; low-battery-only=" + str((current or {}).get("pause_on_low_battery_only")))
        result = self.prompt(
            "lifecycle.battery_unplugged",
            "After unplugging AC where practical, did UPower report OnBattery=yes and did the configured wallpaper policy take effect?",
            "Do not run a battery down for this test. Reconnect AC after checking. The harness reads UPower before and after your action.",
            mandatory=True,
        )
        unplugged = self.upower_snapshot()
        self.report["diagnostics"]["upower_unplugged"] = unplugged
        unplugged_status = self.get_status()
        configured = bool((unplugged_status or {}).get("pause_on_battery"))
        low_only = bool((unplugged_status or {}).get("pause_on_low_battery_only"))
        percentage = unplugged.get("percentage")
        percentage_allows_pause = not low_only or (percentage is not None and percentage <= 20.0)
        expected_reason = configured and unplugged.get("on_battery") is True and percentage_allows_pause
        observed_reason = "on-battery" in ((unplugged_status or {}).get("pause_reasons") or [])
        policy_ok = expected_reason == observed_reason
        if result == "PASS" and unplugged.get("on_battery") is True and policy_ok:
            self.check("lifecycle.battery_policy", "PASS", {
                "before": before["on_battery"],
                "after_unplug": unplugged["on_battery"],
                "configured_pause_on_battery": configured,
                "low_battery_only": low_only,
                "upower_percentage": percentage,
                "renderer_status": unplugged_status,
            })
        elif low_only and percentage is None:
            self.check("lifecycle.battery_policy", "UNAVAILABLE", {
                "reason": "Low-battery-only is enabled but UPower percentage could not be read",
                "upower": unplugged,
                "renderer_status": unplugged_status,
            }, mandatory=True)
        elif result == "SKIPPED":
            self.check("lifecycle.battery_policy", "SKIPPED", "Battery transition was not exercised", mandatory=True)
        else:
            self.check("lifecycle.battery_policy", "FAIL", {"manual": result, "UPower": unplugged.get("on_battery")})
        self.ask_continue("Reconnect AC power now, then return to this terminal.")
        plugged = self.upower_snapshot()
        self.report["diagnostics"]["upower_plugged"] = plugged
        plugged_status = self.get_status()
        if plugged.get("on_battery") is False and plugged_status and "on-battery" not in plugged_status.get("pause_reasons", []):
            self.check("lifecycle.battery_ac_restore", "PASS", {"upower": plugged, "renderer_status": plugged_status}, mandatory=True)
        else:
            self.check("lifecycle.battery_ac_restore", "FAIL", {"upower": plugged, "renderer_status": plugged_status}, mandatory=True)
        self.check("lifecycle.low_battery_threshold", "UNAVAILABLE",
                   "A safe low-charge threshold transition was not induced; no battery level was simulated", mandatory=False)

    def suspend_available(self) -> bool | None:
        result = command(["loginctl", "can-suspend"])
        if result["status"] != 0:
            return None
        if result["stdout"].strip().lower() == "yes":
            return True
        if result["stdout"].strip().lower() in {"no", "na"}:
            return False
        return None

    def record_status_check(self, key: str, status: dict[str, Any] | None,
                            predicate, description: str, mandatory: bool = True) -> bool:
        passed = bool(status and predicate(status))
        self.check(key, "PASS" if passed else "FAIL", status or description, mandatory=mandatory)
        return passed

    def sample_performance(self, label: str, duration: int = 10) -> dict[str, Any] | None:
        pids = binary_processes("gnomeengine-renderer")
        if len(pids) != 1:
            evidence = {"renderer_pids": pids, "reason": "expected exactly one renderer process"}
            self.report["performance"][label] = evidence
            self.check(f"performance.{label}", "UNAVAILABLE", evidence, mandatory=True)
            return None
        pid = pids[0]

        def read_proc() -> tuple[int, int] | None:
            try:
                raw = (Path("/proc") / str(pid) / "stat").read_text()
                fields = raw[raw.rfind(")") + 2:].split()
                ticks = int(fields[11]) + int(fields[12])
                status = (Path("/proc") / str(pid) / "status").read_text()
                rss_match = re.search(r"^VmRSS:\s+(\d+)\s+kB", status, re.MULTILINE)
                if not rss_match:
                    return None
                return ticks, int(rss_match.group(1))
            except (OSError, ValueError, IndexError):
                return None

        tick_hz = os.sysconf("SC_CLK_TCK")
        first = read_proc()
        if first is None:
            self.check(f"performance.{label}", "UNAVAILABLE", "Could not read /proc renderer counters", mandatory=True)
            return None
        rss_values = [first[1]]
        start = time.monotonic()
        for _ in range(duration):
            time.sleep(1)
            current = read_proc()
            if current is None:
                self.check(f"performance.{label}", "UNAVAILABLE", "Renderer exited during the sample", mandatory=True)
                return None
            rss_values.append(current[1])
        elapsed = time.monotonic() - start
        cpu_percent = max(0.0, (current[0] - first[0]) / tick_hz / elapsed * 100)
        sample = {
            "duration_seconds": round(elapsed, 2),
            "pid": pid,
            "cpu_percent_one_core": round(cpu_percent, 2),
            "rss_kib_average": round(sum(rss_values) / len(rss_values), 1),
            "rss_kib_min": min(rss_values),
            "rss_kib_max": max(rss_values),
        }
        self.report["performance"][label] = sample
        self.check(f"performance.{label}", "PASS", sample, mandatory=True)
        self.save()
        return sample

    def perform_performance_samples(self) -> None:
        status = self.get_status()
        current_video = status.get("current_video") if status else None
        if not status or status.get("state") != "playing" or status.get("pause_reasons"):
            evidence = {"status": status, "reason": "Playing sample needs state=playing and no pause reasons; connect AC/leave fullscreen if needed"}
            self.report["performance"]["playing"] = evidence
            self.check("performance.playing", "UNAVAILABLE", evidence, mandatory=True)
        else:
            self.sample_performance("playing", 10)
        pause_ok, pause_text = self.renderer_call("Pause")
        paused = self.wait_status(lambda item: item.get("state") == "paused" and "manual" in item.get("pause_reasons", [])) if pause_ok else None
        if paused:
            self.sample_performance("paused", 10)
        else:
            self.report["performance"]["paused"] = {"pause_call": pause_text, "status": paused}
            self.check("performance.paused", "UNAVAILABLE", self.report["performance"]["paused"], mandatory=True)
        resume_ok, resume_text = self.renderer_call("Resume")
        resumed = self.wait_status(lambda item: item.get("state") in {"playing", "paused"}) if resume_ok else None
        resume_state_ok = bool(resumed and resumed.get("state") == ("paused" if resumed.get("pause_reasons") else "playing"))
        self.check("performance.resume_after_pause", "PASS" if resume_state_ok else "FAIL",
                   {"resume_call": resume_text, "status": resumed}, mandatory=True)
        stop_ok, stop_text = self.renderer_call("Stop")
        stopped = self.wait_status(lambda item: item.get("state") == "stopped" and item.get("wallpaper_active") is False) if stop_ok else None
        if stopped:
            self.sample_performance("stopped", 10)
        else:
            self.report["performance"]["stopped"] = {"stop_call": stop_text, "status": stopped}
            self.check("performance.stopped", "UNAVAILABLE", self.report["performance"]["stopped"], mandatory=True)
        if current_video:
            apply_ok, apply_text = self.renderer_call("ApplyVideo", json.dumps(current_video))
            reapplied = self.wait_status(lambda item: item.get("wallpaper_active") is True and item.get("desktop_integration_ready") is True) if apply_ok else None
            if not reapplied:
                self.report["diagnostics"]["performance_restore"] = {"call": apply_text, "status": reapplied}
                self.check("performance.restore_after_sample", "FAIL", self.report["diagnostics"]["performance_restore"], mandatory=True)
            else:
                self.check("performance.restore_after_sample", "PASS", reapplied, mandatory=True)
        else:
            self.check("performance.restore_after_sample", "FAIL", "No current wallpaper path was available to restore after sampling", mandatory=True)

        playing = self.report.get("performance", {}).get("playing", {})
        paused = self.report.get("performance", {}).get("paused", {})
        if "cpu_percent_one_core" in playing and "cpu_percent_one_core" in paused:
            cheaper = paused["cpu_percent_one_core"] < playing["cpu_percent_one_core"]
            evidence = {"playing_cpu_percent": playing["cpu_percent_one_core"],
                        "paused_cpu_percent": paused["cpu_percent_one_core"],
                        "paused_is_lower": cheaper}
            self.check("performance.paused_cheaper", "PASS" if cheaper else "FAIL", evidence, mandatory=True)
        else:
            self.check("performance.paused_cheaper", "UNAVAILABLE", "Both Playing and Paused CPU samples are required", mandatory=True)

    def capture_gstreamer_summary(self, shell_log: str, renderer_log: str) -> None:
        combined = shell_log + "\n" + renderer_log
        sink = None
        if re.search(r"selected GStreamer gtk4paintablesink", combined, re.IGNORECASE):
            if "registering bundled GTK4 GStreamer sink" in combined:
                sink = "gtk4paintablesink selected; private registration observed"
            else:
                sink = "gtk4paintablesink selected; registration origin not distinguished in logs"
        elif re.search(r"selected GTK media playback fallback|selected GTK media playback|using GTK media backend", combined, re.IGNORECASE):
            sink = "GTK media backend fallback selected"
        decoder_matches = re.findall(r"(?:using|selected|decoder)\s+[^\n]*decoder[^\n]*", combined, re.IGNORECASE)
        explicit_dma = re.search(
            r"(?:DMA[- ]BUF[^\n]*(?:path active|active path|imported|in use|using|confirmed)|"
            r"(?:active|imported|in use|using|confirmed)[^\n]*DMA[- ]BUF)",
            combined,
            re.IGNORECASE,
        )
        graphics = re.findall(r"INFO renderer (?:graphics runtime|(?:X11|Wayland) path):[^\n]*", combined)
        summary = {
            "sink": sink,
            "decoder": decoder_matches[-1] if decoder_matches else None,
            "graphics_path_diagnostic": graphics[-1] if graphics else None,
            "dmabuf_confirmed": bool(explicit_dma),
            "dmabuf_evidence": explicit_dma.group(0) if explicit_dma else None,
            "media": self.report.get("diagnostics", {}).get("media", {}),
        }
        self.report["diagnostics"]["gstreamer"] = summary
        self.check("gstreamer.sink_observed", "PASS" if sink else "UNAVAILABLE",
                   summary if sink else "Playback may be visible, but logs did not identify the selected sink", mandatory=True)
        self.check("gstreamer.decoder_observed", "PASS" if decoder_matches else "UNAVAILABLE",
                   summary["decoder"] or "No decoder selection log was observed", mandatory=False)
        self.save()

    def pause_signal_duplicates(self) -> None:
        path = self.json_path.parent / "gdbus-monitor.log"
        if not path.exists():
            self.check("renderer.pause_reason_signals", "UNAVAILABLE", "D-Bus signal monitor did not produce a log", mandatory=True)
            return
        text = path.read_text(errors="replace")
        events = []
        for line in text.splitlines():
            if "PauseReasonsChanged" not in line:
                continue
            match = re.search(r"PauseReasonsChanged[^\[]*\[([^\]]*)\]", line)
            if match:
                values = re.findall(r"'([^']*)'|\"([^\"]*)\"", match.group(1))
                events.append(tuple(sorted(value for pair in values for value in pair if value)))
        normalized = events
        duplicates = [normalized[i] for i in range(1, len(normalized)) if normalized[i] == normalized[i - 1]]
        if not events:
            self.check("renderer.pause_reason_signals", "UNAVAILABLE", "No PauseReasonsChanged signals were observed", mandatory=True)
        else:
            self.check("renderer.pause_reason_signals", "FAIL" if duplicates else "PASS",
                       {"event_count": len(events), "consecutive_duplicate_reason_sets": duplicates}, mandatory=True)

    def bridge_check(self, shell_log: str) -> None:
        if self.target == "ubuntu24.04-gnome46-wayland":
            expected = "renderer surface attached through the GNOME 46 Wayland client bridge"
            mode = "legacy-wayland / Meta.WaylandClient"
            passed = expected in shell_log
        elif self.target == "ubuntu24.04-gnome46-x11":
            expected = "renderer surface recognized with X11 EWMH desktop hints"
            mode = "x11-ewmh / GDK X11 surface"
            passed = expected in shell_log
        else:
            expected = "renderer surface attached as a desktop window"
            mode = "modern-wayland / Meta.Window"
            passed = expected in shell_log and "GNOME 46 Wayland client bridge" not in shell_log
        self.report["diagnostics"]["selected_bridge"] = {
            "expected": mode,
            "evidence": expected if passed else None,
            "legacy_marker_seen": "GNOME 46 Wayland client bridge" in shell_log,
        }
        self.check("bridge.selected", "PASS" if passed else "FAIL", self.report["diagnostics"]["selected_bridge"], mandatory=True)

    def monitor_manual(self) -> None:
        count = self.x11_active_monitor_count()
        available = count is not None and count > 1
        if not available:
            self.check("desktop.monitor_topology_change", "UNAVAILABLE",
                       {"monitor_count": count, "reason": "No multiple active monitors were detected"}, mandatory=False)
            return
        self.prompt("desktop.monitor_topology_change",
                    "After connecting or disconnecting one monitor, did wallpaper geometry and playback recover with no duplicate surface/crash?",
                    f"Detected active monitors: {count}. Recheck the renderer D-Bus state after changing topology.", mandatory=True)
        status = self.get_status()
        pids = self.process_counts()["gnomeengine-renderer"]
        passed = bool(status and status.get("wallpaper_active") and len(pids) == 1)
        self.check("desktop.monitor_topology_automated", "PASS" if passed else "FAIL",
                   {"status": status, "renderer_pids": pids}, mandatory=True)

    def restart_test(self) -> None:
        old_pids = binary_processes("gnomeengine-renderer")
        if len(old_pids) != 1:
            self.check("lifecycle.renderer_restart", "UNAVAILABLE",
                       {"renderer_pids_before": old_pids, "reason": "exactly one renderer must be active"}, mandatory=True)
            return
        pid = old_pids[0]
        before = self.get_status()
        current_video = before.get("current_video") if before else None
        self.ask_continue("For a lifecycle-snapshot check, open a real app fullscreen on the primary monitor, then switch to another workspace while leaving it fullscreen.")
        fullscreen = self.wait_status(lambda item: "fullscreen" in item.get("pause_reasons", []), timeout=30)
        if not fullscreen or not current_video:
            self.check("lifecycle.renderer_restart_fullscreen_setup", "FAIL",
                       {"status": fullscreen, "current_video_available": bool(current_video)}, mandatory=True)
            self.check("lifecycle.renderer_restart", "FAIL", "Could not establish a fullscreen reason before renderer restart", mandatory=True)
            return
        self.check("lifecycle.renderer_restart_fullscreen_setup", "PASS", fullscreen, mandatory=True)
        self.prompt(
            "lifecycle.renderer_restart_manual",
            "After the renderer restart, did it return paused while fullscreen remained active, then recover after fullscreen ended?",
            f"In a second terminal run `kill -TERM {pid}` as this user only. On the other workspace, use Apply for the same wallpaper in GnomeEngine. Do not use sudo. Keep the fullscreen app active until the harness confirms the new status.",
            mandatory=True,
        )
        deadline = time.monotonic() + 45
        status = None
        new_pids: list[int] = []
        while time.monotonic() < deadline:
            new_pids = binary_processes("gnomeengine-renderer")
            status = self.get_status()
            if (len(new_pids) == 1 and new_pids[0] != pid and status
                    and status.get("wallpaper_active") and status.get("desktop_integration_ready")
                    and status.get("state") == "paused" and "fullscreen" in status.get("pause_reasons", [])):
                break
            time.sleep(1)
        passed = bool(len(new_pids) == 1 and new_pids[0] != pid and status
                      and status.get("wallpaper_active") and status.get("desktop_integration_ready")
                      and status.get("state") == "paused" and "fullscreen" in status.get("pause_reasons", []))
        self.check("lifecycle.renderer_restart", "PASS" if passed else "FAIL",
                   {"old_pids": old_pids, "new_pids": new_pids, "status": status}, mandatory=True)
        self.check("lifecycle.renderer_restart_snapshot", "PASS" if passed else "FAIL",
                   {"fullscreen_present_after_restart": bool(status and "fullscreen" in status.get("pause_reasons", [])),
                    "status": status}, mandatory=True)
        self.ask_continue("Switch back to the fullscreen workspace and exit fullscreen.")
        after_exit = self.wait_status(lambda item: "fullscreen" not in item.get("pause_reasons", []), timeout=30)
        expected = "paused" if after_exit and after_exit.get("pause_reasons") else "playing"
        recovery = bool(after_exit and after_exit.get("state") == expected and after_exit.get("wallpaper_active") is True)
        self.check("lifecycle.renderer_restart_recovery", "PASS" if recovery else "FAIL",
                   {"status": after_exit, "expected_state": expected}, mandatory=True)
        self.no_duplicates_check()

    def no_duplicates_check(self) -> None:
        pids = binary_processes("gnomeengine-renderer")
        passed = len(pids) <= 1
        self.report["diagnostics"]["renderer_process_pids"] = pids
        self.check("renderer.no_duplicate_processes", "PASS" if passed else "FAIL", {"pids": pids}, mandatory=True)

    def gui_launch(self) -> bool:
        previous = binary_processes("gnomeengine")
        if previous:
            print(f"Existing GnomeEngine GUI process(es): {previous}. Close them before this fresh launch.")
            deadline = time.monotonic() + 60
            while binary_processes("gnomeengine") and time.monotonic() < deadline:
                time.sleep(1)
            if binary_processes("gnomeengine"):
                self.check("application.gui_visible", "FAIL", "A previous GnomeEngine GUI remained open")
                return False
        app_path = shutil.which("gnomeengine")
        if not app_path:
            self.check("application.gui_visible", "FAIL", "Installed gnomeengine executable not found in PATH")
            return False
        env = os.environ.copy()
        removed = []
        for key in ("GSETTINGS_SCHEMA_DIR", "GST_PLUGIN_PATH", "GST_PLUGIN_PATH_1_0", "GST_PLUGIN_SYSTEM_PATH",
                    "GST_PLUGIN_SYSTEM_PATH_1_0", "LD_LIBRARY_PATH", "GI_TYPELIB_PATH", "GIO_EXTRA_MODULES", "GTK_PATH"):
            if key in env:
                env.pop(key)
                removed.append(key)
        repo_path_components = []
        for key in ("PATH", "XDG_DATA_DIRS"):
            if key not in env:
                continue
            retained = []
            for entry in env[key].split(os.pathsep):
                try:
                    resolved = Path(entry or ".").expanduser().resolve()
                    in_repo = resolved == ROOT or ROOT in resolved.parents
                except OSError:
                    in_repo = False
                if in_repo:
                    repo_path_components.append({"variable": key, "path": entry})
                else:
                    retained.append(entry)
            env[key] = os.pathsep.join(retained)
        self.report["diagnostics"]["removed_development_environment_variables"] = removed
        self.report["diagnostics"]["removed_repository_path_entries"] = repo_path_components
        log_path = self.json_path.parent / "application.log"
        stream = log_path.open("w")
        self.app_process = subprocess.Popen([app_path], cwd=Path.home(), env=env, stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        self.gui_pid = self.app_process.pid
        self.report["diagnostics"]["application_pid"] = self.gui_pid
        self.report["diagnostics"]["application_log"] = str(log_path)
        stream.close()
        time.sleep(3)
        return True

    def apply_initial(self) -> dict[str, Any] | None:
        status = self.prompt(
            "application.import_preview_apply",
            "Did you import the provided local video, open its detail/preview, and Apply Wallpaper in the installed GUI?",
            f"Import/select this exact local file in the Library: {self.video}\nWait for preview, then click Apply Wallpaper. Do not run cargo from the checkout.",
            mandatory=True,
        )
        if status != "PASS":
            self.check("renderer.apply_dbus_status", "SKIPPED", "The GUI import/preview/Apply step did not pass")
            return None
        deadline = time.monotonic() + 35
        renderer_status = None
        while time.monotonic() < deadline:
            if self.renderer_owner() is True:
                renderer_status = self.get_status()
                if renderer_status and renderer_status.get("wallpaper_active") is True:
                    break
            time.sleep(1)
        passed = bool(renderer_status and renderer_status.get("state") in {"playing", "paused"}
                      and renderer_status.get("wallpaper_active") is True
                      and renderer_status.get("desktop_integration_ready") is True)
        self.report["diagnostics"]["renderer_after_apply"] = renderer_status
        self.check("renderer.apply_dbus_status", "PASS" if passed else "FAIL",
                   renderer_status or "Renderer service/status unavailable after Apply", mandatory=True)
        if not passed:
            return renderer_status
        self.start_signal_monitor()
        shell, renderer = self.save_session_logs()
        self.bridge_check(shell)
        self.capture_gstreamer_summary(shell, renderer)
        self.geometry_from_logs(shell)
        self.x11_diagnostics()
        self.no_duplicates_check()
        return renderer_status

    def desktop_manual_checks(self) -> None:
        checks = [
            ("desktop.wallpaper_visible", "Is the moving wallpaper visible on the desktop?", "Show the desktop with no normal window covering it."),
            ("desktop.not_player_window", "Does the wallpaper appear as desktop content rather than as a normal GTK/player window?", "Inspect the desktop and window decorations while opening a regular application."),
            ("desktop.behind_windows", "Does the wallpaper stay behind normal application windows?", "Open and move a normal window over the desktop."),
            ("desktop.alt_tab", "Is the renderer absent from Alt+Tab?", "Open Alt+Tab and inspect every application entry."),
            ("desktop.overview", "Is the renderer absent from GNOME Overview as an application window?", "Open Overview with Super and inspect windows and app entries."),
            ("desktop.mouse_input", "Does mouse interaction pass through the wallpaper without being consumed?", "Click desktop icons, right-click the desktop, and use a normal window."),
            ("desktop.keyboard_focus", "Does the wallpaper avoid stealing keyboard focus?", "Type into a normal application after applying the wallpaper."),
            ("desktop.panel_stacking", "Does the GNOME panel/dock render above the wallpaper?", "Inspect the whole panel/dock, including its reserved area."),
            ("desktop.notifications_stacking", "Do GNOME notifications render above the wallpaper?", "Trigger a normal notification and inspect its stacking."),
            ("desktop.system_dialogs_stacking", "Do system dialogs render above the wallpaper?", "Open a harmless system dialog such as the date/time menu or Settings dialog."),
            ("desktop.full_monitor_visual", "Does wallpaper cover the full monitor, including behind the panel/dock?", "Compare the visible wallpaper to the physical monitor edges; do not judge against the smaller work area."),
            ("desktop.no_static_strip", "Is there no uncovered static-wallpaper strip around the panel/dock?", "Inspect the full edge around panels/docks for any exposed static background."),
            ("desktop.workspaces", "Does wallpaper behave correctly while switching workspaces?", "Switch through all available workspaces and return to the original one."),
        ]
        for key, question, instructions in checks:
            self.prompt(key, question, instructions)

    def cycle_tests(self) -> None:
        all_pass = True
        for cycle in range(1, 4):
            stopped_manual = self.prompt(
                f"renderer.cycle_{cycle}_stop_visual",
                f"Cycle {cycle}: after clicking Stop in GnomeEngine, did the wallpaper disappear cleanly?",
                "Use the installed GUI Stop button; do not kill the renderer for this cycle.",
                mandatory=True,
            )
            stopped = self.wait_status(lambda item: item.get("state") == "stopped" and item.get("wallpaper_active") is False)
            stopped_ok = stopped_manual == "PASS" and bool(stopped and stopped.get("state") == "stopped" and stopped.get("wallpaper_active") is False)
            self.check(f"renderer.cycle_{cycle}_stop_state", "PASS" if stopped_ok else "FAIL", stopped or "No D-Bus status after Stop", mandatory=True)
            applied_manual = self.prompt(
                f"renderer.cycle_{cycle}_apply_visual",
                f"Cycle {cycle}: after applying the same video again, did the wallpaper return?",
                "Use the installed GUI Apply action and confirm the desktop visually.",
                mandatory=True,
            )
            active = self.wait_status(lambda item: item.get("wallpaper_active") is True and item.get("desktop_integration_ready") is True)
            active_ok = applied_manual == "PASS" and bool(active and active.get("state") in {"playing", "paused"})
            self.check(f"renderer.cycle_{cycle}_apply_state", "PASS" if active_ok else "FAIL", active or "No active renderer status after Apply", mandatory=True)
            all_pass = all_pass and stopped_ok and active_ok
            self.no_duplicates_check()
        self.check("renderer.apply_stop_cycles", "PASS" if all_pass else "FAIL",
                   {"cycles": 3, "completed": all_pass}, mandatory=True)

    def close_reopen_gui(self) -> None:
        self.prompt("application.close_keeps_wallpaper_manual", "After closing the GnomeEngine GUI, is the wallpaper still active?",
                    "Close the main application window normally while the wallpaper is active; do not choose Stop.", mandatory=True)
        time.sleep(3)
        gui_pids = binary_processes("gnomeengine")
        status = self.get_status()
        gui_exited = not gui_pids
        wallpaper_active = bool(status and status.get("wallpaper_active") is True)
        self.check("application.close_keeps_wallpaper", "PASS" if gui_exited and wallpaper_active else "FAIL",
                   {"gui_pids_after_close": gui_pids, "renderer_status": status}, mandatory=True)
        if not self.gui_launch():
            self.check("application.reopen_status", "FAIL", "Installed GUI did not relaunch")
            return
        self.prompt("application.reopen_gui_visible", "Did the installed GUI reopen successfully?", mandatory=True)
        status = self.get_status()
        manual = self.prompt("application.reopen_status_manual", "Does the reopened GUI show the correct active wallpaper and state?",
                             "Compare its active state/current item with the renderer status captured by the harness.", mandatory=True)
        passed = bool(manual == "PASS" and status and status.get("wallpaper_active") is True
                      and status.get("state") in {"playing", "paused"})
        self.check("application.reopen_status", "PASS" if passed else "FAIL", status or "Renderer D-Bus status unavailable", mandatory=True)

    def lifecycle_fullscreen(self) -> None:
        initial = self.get_status()
        if not initial or initial.get("wallpaper_active") is not True:
            self.check("lifecycle.fullscreen_pause", "FAIL", "No active renderer before fullscreen test")
            self.check("lifecycle.fullscreen_resume", "SKIPPED", "No active renderer before fullscreen test")
            self.check("lifecycle.pause_reason_composition", "SKIPPED", "No active renderer before pause-reason test")
            self.check("lifecycle.apply_while_fullscreen_paused", "SKIPPED", "No active renderer before fullscreen test")
            return
        original_video = initial.get("current_video")
        pause_ok, pause_result = self.renderer_call("Pause")
        manual_paused = self.wait_status(lambda item: item.get("state") == "paused" and "manual" in item.get("pause_reasons", [])) if pause_ok else None
        manual_ok = bool(manual_paused and "manual" in manual_paused.get("pause_reasons", []))
        self.check("lifecycle.manual_pause", "PASS" if manual_ok else "FAIL",
                   manual_paused or pause_result, mandatory=True)
        self.ask_continue("Now open another real application fullscreen on the primary monitor. Leave it fullscreen until the following checks finish.")
        fullscreen = self.wait_status(lambda item: "fullscreen" in item.get("pause_reasons", []), timeout=30)
        fullscreen_ok = bool(fullscreen and fullscreen.get("state") == "paused" and "manual" in fullscreen.get("pause_reasons", []))
        self.check("lifecycle.fullscreen_pause", "PASS" if fullscreen_ok else "FAIL", fullscreen or "Fullscreen reason not observed", mandatory=True)
        resume_ok, resume_text = self.renderer_call("Resume")
        after_manual_resume = self.wait_status(lambda item: "manual" not in item.get("pause_reasons", [])) if resume_ok else None
        reason_composed = bool(after_manual_resume and "fullscreen" in after_manual_resume.get("pause_reasons", [])
                               and after_manual_resume.get("state") == "paused")
        self.check("lifecycle.pause_reason_composition", "PASS" if reason_composed else "FAIL",
                   after_manual_resume or resume_text, mandatory=True)
        stop_ok, stop_message = self.renderer_call("Stop")
        stopped = self.wait_status(lambda item: item.get("state") == "stopped" and item.get("wallpaper_active") is False) if stop_ok else None
        self.check("lifecycle.stop_while_auto_paused", "PASS" if stopped else "FAIL",
                   stopped or stop_message, mandatory=True)
        self.ask_continue("Exit fullscreen now and return to the desktop. The renderer must remain Stopped.")
        stopped_after_exit = self.wait_status(lambda item: "fullscreen" not in item.get("pause_reasons", []), timeout=30)
        stays_stopped = bool(stopped_after_exit and stopped_after_exit.get("state") == "stopped"
                             and stopped_after_exit.get("wallpaper_active") is False)
        self.check("lifecycle.stop_stays_stopped_after_condition", "PASS" if stays_stopped else "FAIL",
                   stopped_after_exit, mandatory=True)
        self.ask_continue("Open a real application fullscreen on the primary monitor once more.")
        fullscreen_again = self.wait_status(lambda item: "fullscreen" in item.get("pause_reasons", []), timeout=30)
        if original_video and fullscreen_again:
            apply_ok, apply_message = self.renderer_call("ApplyVideo", json.dumps(original_video))
            applied = self.wait_status(lambda item: item.get("state") == "paused" and item.get("wallpaper_active") is True
                                       and "fullscreen" in item.get("pause_reasons", [])) if apply_ok else None
            self.check("lifecycle.apply_while_fullscreen_paused", "PASS" if applied else "FAIL",
                       applied or {"fullscreen": fullscreen_again, "apply": apply_message}, mandatory=True)
        else:
            self.check("lifecycle.apply_while_fullscreen_paused", "UNAVAILABLE",
                       {"status": fullscreen_again, "reason": "No fullscreen condition was available for re-Apply"}, mandatory=True)
        self.ask_continue("Exit fullscreen now and return to the desktop.")
        after_exit = self.wait_status(lambda item: "fullscreen" not in item.get("pause_reasons", []), timeout=30)
        reasons = after_exit.get("pause_reasons", []) if after_exit else []
        expected_state = "paused" if reasons else "playing"
        resume_state_ok = bool(after_exit and after_exit.get("state") == expected_state and "fullscreen" not in reasons)
        self.check("lifecycle.fullscreen_resume", "PASS" if resume_state_ok else "FAIL",
                   {"status": after_exit, "expected_state": expected_state}, mandatory=True)
        self.prompt("lifecycle.pause_reason_composition_manual",
                    "Did Manual Resume leave playback paused during fullscreen, then recover correctly after fullscreen ended?",
                    "This checks the visible result of the reason-composition sequence above.", mandatory=True)

    def lifecycle_lock_suspend(self) -> None:
        before_lock = self.get_status()
        self.ask_continue("Lock the real desktop with Super+L, unlock it, then return to this terminal.")
        self.prompt("lifecycle.lock_unlock_manual", "After unlock, did the wallpaper recover without a black desktop or duplicate renderer?", mandatory=True)
        after_lock = self.get_status()
        pids = binary_processes("gnomeengine-renderer")
        lock_ok = bool(after_lock and after_lock.get("wallpaper_active") is True
                       and "screen-locked" not in after_lock.get("pause_reasons", []) and len(pids) == 1)
        self.check("lifecycle.lock_unlock", "PASS" if lock_ok else "FAIL",
                   {"before": before_lock, "after": after_lock, "renderer_pids": pids}, mandatory=True)
        suspend = self.suspend_available()
        self.report["diagnostics"]["suspend_available"] = suspend
        if suspend is False:
            self.check("lifecycle.suspend_resume", "UNAVAILABLE", "loginctl can-suspend reports no; this environment cannot suspend", mandatory=False)
            return
        if suspend is None:
            self.check("lifecycle.suspend_resume", "UNAVAILABLE", "Suspend support could not be determined", mandatory=True)
            return
        before_suspend = self.get_status()
        # The complete report is atomically saved immediately before the potentially long system sleep.
        self.report["diagnostics"]["suspend_checkpoint"] = {"saved_at": now(), "renderer_status": before_suspend}
        self.save()
        self.ask_continue("Suspend through GNOME's system menu or power menu, resume the same session, then return here. The harness does not invoke suspend.")
        self.prompt("lifecycle.suspend_visual", "After resume, is the desktop visible and the wallpaper recovered?", mandatory=True)
        after_suspend = self.get_status()
        pids = binary_processes("gnomeengine-renderer")
        suspend_ok = bool(after_suspend and after_suspend.get("wallpaper_active") is True
                          and "system-sleep" not in after_suspend.get("pause_reasons", []) and len(pids) == 1)
        self.check("lifecycle.suspend_resume", "PASS" if suspend_ok else "FAIL",
                   {"before": before_suspend, "after": after_suspend, "renderer_pids": pids}, mandatory=True)

    def display_power_test(self) -> None:
        backlights = [path.name for path in Path("/sys/class/backlight").glob("*")]
        self.report["diagnostics"]["backlights"] = backlights
        if not backlights:
            self.check("lifecycle.display_power", "UNAVAILABLE",
                       "No built-in backlight device is exposed; display power transition is hardware-specific", mandatory=False)
            return
        self.prompt(
            "lifecycle.display_power_manual",
            "After safely turning the built-in display off and back on, did playback pause while off and recover afterward?",
            f"Backlight devices detected: {backlights}. Use the system's normal display power control; do not run a command that leaves the screen off.",
            mandatory=True,
        )
        status = self.get_status()
        log_path = self.json_path.parent / "gdbus-monitor.log"
        signal_text = log_path.read_text(errors="replace") if log_path.exists() else ""
        reason_seen = "display-off" in signal_text
        recovered = bool(status and status.get("wallpaper_active") is True
                         and "display-off" not in status.get("pause_reasons", []))
        passed = reason_seen and recovered
        self.check("lifecycle.display_power", "PASS" if passed else "FAIL",
                   {"backlights": backlights, "display_off_reason_signal_seen": reason_seen, "status_after_display_on": status}, mandatory=True)

    def extension_reload(self) -> None:
        self.prompt(
            "lifecycle.extension_reload_manual",
            "After reloading the extension and applying again, did the wallpaper recover without a stale pause reason or duplicate surface?",
            f"In a second terminal run `gnome-extensions disable {UUID}` then `gnome-extensions enable {UUID}`. Return here, then Apply again in the GUI. Do not uninstall a temporary test copy yet.",
            mandatory=True,
        )
        details = self.extension_info()
        status = self.get_status()
        pids = binary_processes("gnomeengine-renderer")
        forbidden = {"screen-locked", "system-sleep"}
        passed = bool(details["state"] == "ENABLED" and status and status.get("wallpaper_active") is True
                      and not (forbidden & set(status.get("pause_reasons", []))) and len(pids) == 1)
        self.check("lifecycle.extension_reload", "PASS" if passed else "FAIL",
                   {"extension": details, "renderer_status": status, "renderer_pids": pids}, mandatory=True)

    def run_acceptance_sequence(self) -> bool:
        env = self.environment
        matched = self.target is not None
        self.check("environment.target_match", "PASS" if matched else "FAIL",
                   self.target or {"observed": {"os": env["os"], "gnome": env["gnome"], "session": env.get("session_type"), "desktop": env.get("current_desktop")}})
        valid_session = real_gnome_session(env)
        self.check("environment.real_gnome_session", "PASS" if valid_session else "FAIL",
                   {"process_pids": env["gnome_shell_process"]["pids"], "session_bus": env["session_bus"]})
        if not matched:
            print("UNSUPPORTED TEST ENVIRONMENT")
            return False
        if not valid_session:
            print("THIS IS NOT A VALID M7 RUNTIME ACCEPTANCE SESSION")
            return False
        contaminated = bool(env["development_environment_overrides"] or env["repository_path_entries"])
        self.check("environment.no_development_overrides", "UNAVAILABLE" if contaminated else "PASS",
                   {"overrides": env["development_environment_overrides"],
                    "repository_path_entries": env["repository_path_entries"]}, mandatory=not contaminated)
        if contaminated:
            print("Development/repository environment overrides are present in this GNOME session.")
            print("Log out, remove those overrides from the session environment, log in again, and rerun acceptance.")
            return False

        self.collect_local_checks()
        deb_value = self._deb_argument
        installed = self.installed_package()
        if not installed:
            deb_text = str(Path(deb_value).expanduser().resolve()) if deb_value else "./<package>.deb"
            print("\nTo perform package acceptance, run:")
            print(f"sudo apt install {deb_text}")
            print("Then rerun:")
            rerun = f"./scripts/m7-acceptance.sh --deb {shlex.quote(deb_text)}"
            if self._video_argument:
                rerun += f" --video {shlex.quote(str(Path(self._video_argument).expanduser()))}"
            print(rerun)
            return False
        candidate_match = self.report["package"].get("candidate_matches_installed", True)
        if candidate_match is False:
            print("The supplied .deb is not the installed version; runtime results would not validate that candidate.")
            print("Install that package explicitly, then rerun this script. No package install was attempted.")
            return False

        video_ready = self.video_precheck(self._video_argument)
        if not video_ready:
            print("Precheck failure: a local video must be readable and discoverable by GStreamer.")
            return False
        extension_ready = self.ensure_extension_enabled()
        if not extension_ready:
            print("Precheck failure: the packaged extension is not enabled in this target session.")
            return False
        automated = self.report["automated"]
        prechecks = {
            "target": self.target,
            "real_session": valid_session,
            "installed_package": installed,
            "video": self.report["checks"].get("media.discovered", {}).get("status"),
            "extension": self.report["checks"].get("extension.enabled", {}).get("status"),
            "session_bus": env["session_bus"]["reachable"],
            "source_package_check": automated.get("source_package_check", {}).get("status"),
            "workspace_check": automated.get("workspace_check", {}).get("status"),
            "apt_dependency_simulation": self.report["package"].get("dependency_simulation", {}).get("status", "NOT RUN"),
        }
        self.report["automated"]["prechecks"] = prechecks
        self.save()
        print("\nAutomated prechecks:")
        for name, value in prechecks.items():
            print(f"  {name}: {value}")
        package_note = self.report["package"].get("candidate_gnome_dependency_note")
        if package_note:
            print(f"  PACKAGE NOTE: {package_note}")
        print("\nThe renderer is checked only after Apply. Lack of a renderer D-Bus owner before Apply is expected.")
        print(f"Installed app: {self.report['package'].get('installed_application_path')}")
        print(f"Renderer: /usr/libexec/gnomeengine/gnomeengine-renderer")

        if not self.gui_launch():
            return False
        self.report["runtime_started"] = True
        self.save()
        visible = self.prompt("application.gui_visible", "Is the GnomeEngine GUI visible with the correct application icon?", mandatory=True)
        dock = self.prompt("application.single_dock_entry", "Is there exactly one GnomeEngine entry in the Dock?", mandatory=True)
        if visible != "PASS":
            return False
        self.report["diagnostics"]["renderer_owner_before_apply"] = self.renderer_owner()
        apply_status = self.apply_initial()
        if not apply_status or apply_status.get("wallpaper_active") is not True:
            return False
        self.desktop_manual_checks()
        self.cycle_tests()
        self.close_reopen_gui()
        self.lifecycle_fullscreen()
        self.battery_test()
        self.lifecycle_lock_suspend()
        self.display_power_test()
        self.monitor_manual()
        self.restart_test()
        self.extension_reload()
        self.perform_performance_samples()
        self.prompt("renderer.stop_visual", "After clicking Stop in the GUI, did the moving wallpaper disappear cleanly?", mandatory=True)
        stop_ok, stop_output = self.renderer_call("Stop")
        stopped = self.wait_status(lambda item: item.get("state") == "stopped" and item.get("wallpaper_active") is False) if stop_ok else None
        stop_pass = bool(stopped and stopped.get("state") == "stopped" and stopped.get("wallpaper_active") is False)
        self.check("renderer.stop_state", "PASS" if stop_pass else "FAIL", stopped or stop_output, mandatory=True)
        self.no_duplicates_check()
        self.stop_signal_monitor()
        self.pause_signal_duplicates()
        shell, renderer = self.save_session_logs()
        # Re-evaluate bridge and sink after all lifecycle events in case logs arrived late.
        self.bridge_check(shell)
        self.capture_gstreamer_summary(shell, renderer)
        self.geometry_from_logs(shell)
        self.save()
        return True

    def run(self, deb: str | None, video: str | None) -> int:
        self._deb_argument = deb
        self._video_argument = video
        self.print_env()
        if deb:
            self.audit_deb(Path(deb).expanduser().resolve())
        # Unsupported and non-session runs only collect read-only facts and stop here.
        if self.target is None:
            self.check("environment.target_match", "UNAVAILABLE", "UNSUPPORTED TEST ENVIRONMENT", mandatory=False)
            print("UNSUPPORTED TEST ENVIRONMENT")
            self.save()
            print("Overall: INCOMPLETE (no runtime acceptance was run)")
            print(f"JSON: {self.json_path}")
            print(f"Markdown: {self.md_path}")
            return 2
        if not real_gnome_session(self.environment):
            self.check("environment.target_match", "PASS", self.target)
            self.check("environment.real_gnome_session", "UNAVAILABLE", self.environment["session_bus"], mandatory=False)
            print("THIS IS NOT A VALID M7 RUNTIME ACCEPTANCE SESSION")
            print("No runtime result was declared. Use a logged-in GNOME session with a running Shell and reachable session bus.")
            self.save()
            print("Overall: INCOMPLETE (no runtime acceptance was run)")
            print(f"JSON: {self.json_path}")
            print(f"Markdown: {self.md_path}")
            return 2
        try:
            completed = self.run_acceptance_sequence()
            self.report["sequence_complete"] = completed
        except KeyboardInterrupt:
            print("\nInterrupted; the last checkpoint remains in the result file.")
            self.report["diagnostics"]["interrupted"] = True
        finally:
            self.stop_signal_monitor()
            self.save()
        print(f"\nTarget result: {self.report['overall']}")
        print(f"JSON: {self.json_path}")
        print(f"Markdown: {self.md_path}")
        return 0 if self.report["overall"] == "PASS" else 1


def acceptance_main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Run the guided, real-session GnomeEngine M7 acceptance sequence.",
        epilog="The runner never installs packages or downloads test media. An empty manual answer is never PASS.",
    )
    parser.add_argument("--output", help="result directory or exact .json result path")
    parser.add_argument("--deb", help="local .deb candidate to inspect and compare with the installed version")
    parser.add_argument("--video", help="readable local MP4/video file; it is not copied by the harness")
    args = parser.parse_args(argv)
    runner = Acceptance(args.output)
    return runner.run(args.deb, args.video)


def read_result(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    try:
        data = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        return None, str(error)
    return data, None


def result_path(root: Path, target: str) -> Path:
    return root / target / "acceptance.json"


def report_main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Summarize the three M7 acceptance result files.")
    parser.add_argument("--root", default=str(ROOT / "artifacts" / "m7"), help="acceptance artifact root (default: artifacts/m7)")
    parser.add_argument("results", nargs="*", help="optional result.json files or result directories")
    args = parser.parse_args(argv)
    root = Path(args.root).expanduser().resolve()
    supplied: dict[str, list[Path]] = {target: [] for target, _ in MATRIX}
    if args.results:
        for item in args.results:
            candidate = Path(item).expanduser().resolve()
            if candidate.is_dir():
                candidate = candidate / "acceptance.json"
            data, _ = read_result(candidate)
            if data and data.get("target") in supplied:
                supplied[data["target"]].append(candidate)
    else:
        for candidate in root.rglob("acceptance.json") if root.exists() else []:
            data, _ = read_result(candidate)
            if data and data.get("target") in supplied:
                supplied[data["target"]].append(candidate)

    lines = ["# M7 Support Matrix", "", "| Target | Result | Report |", "| --- | --- | --- |"]
    all_present_pass = True
    for target, label in MATRIX:
        paths = supplied[target]
        if not paths:
            result = "MISSING"
            location = "—"
            all_present_pass = False
        else:
            if len(paths) > 1:
                paths.sort(key=lambda path: path.stat().st_mtime if path.exists() else 0)
                path = paths[-1]
            else:
                path = paths[0]
            data, error = read_result(path)
            if not data or data.get("target") != target:
                result = "FAIL"
                all_present_pass = False
            elif data.get("overall") == "PASS" and data.get("sequence_complete") is True:
                result = "PASS"
            elif data.get("overall") == "FAIL":
                result = "FAIL"
                all_present_pass = False
            else:
                result = "MISSING"
                all_present_pass = False
            location = str(path)
            if error:
                location += f" ({error})"
        lines.append(f"| {label} | **{result}** | `{location}` |")
    lines.extend(["", "MISSING means no complete PASS report exists yet. This is a report only; it never edits support metadata.", ""])
    output = "\n".join(lines)
    print(output)
    output_path = root / "support-matrix.md"
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(output)
    print(f"Saved: {output_path}")
    return 0 if all_present_pass else 1


def finalize_main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Require passing reports for every exact M7 target before finalization work.")
    parser.add_argument("--root", default=str(ROOT / "artifacts" / "m7"), help="acceptance artifact root (default: artifacts/m7)")
    args = parser.parse_args(argv)
    root = Path(args.root).expanduser().resolve()
    failures: list[str] = []
    loaded: dict[str, dict[str, Any]] = {}
    print("M7 finalization gate")
    for target, label in MATRIX:
        path = result_path(root, target)
        if not path.is_file() and root.exists():
            candidates = []
            for candidate in root.rglob("acceptance.json"):
                data, _ = read_result(candidate)
                if data and data.get("target") == target:
                    candidates.append(candidate)
            if candidates:
                candidates.sort(key=lambda candidate: candidate.stat().st_mtime)
                path = candidates[-1]
        data, error = read_result(path)
        if not data:
            failures.append(f"{label}: MISSING ({error or path})")
            print(f"  MISSING  {label}")
            continue
        loaded[target] = data
        required = REQUIRED_RUNTIME_CHECKS | conditional_required_checks(data)
        missing = required - data.get("checks", {}).keys()
        failed = [name for name, check in data.get("checks", {}).items()
                  if check.get("mandatory") and check.get("status") != "PASS"]
        valid = (data.get("target") == target and data.get("runner_version") == 1
                 and data.get("sequence_complete") is True and data.get("overall") == "PASS"
                 and not missing and not failed)
        if valid:
            print(f"  PASS     {label}")
        else:
            failures.append(f"{label}: overall={data.get('overall')}, missing={sorted(missing)}, failed={sorted(failed)}")
            print(f"  INCOMPLETE/FAIL  {label}")
            if missing:
                print(f"    missing checks: {', '.join(sorted(missing))}")
            if failed:
                print(f"    mandatory checks not PASS: {', '.join(sorted(failed))}")
    if failures:
        print("\nM7 finalization refused. Keep the M7 plan active and support metadata unchanged.")
        for failure in failures:
            print(f"- {failure}")
        return 1

    lines = [
        "# M7 Runtime Matrix Passed — Final Package Work Still Required",
        "",
        "All three real-session runtime acceptance reports passed.",
        "This gate does not edit or widen source metadata and does not mark M7 complete.",
        "",
        "Before closing M7:",
        "",
        "1. Review the three JSON and Markdown reports plus saved logs.",
        "2. Prepare the exact GNOME 46/50 extension metadata, Debian minimum, and support documentation changes from this evidence.",
        "3. Run canonical source/package checks and rebuild the final package after metadata changes.",
        "4. Install and validate that final package on Ubuntu 24.04 and validate the same package/dependency strategy on Ubuntu 26.04.",
        "5. Record final package evidence and only then move the M7 plan to completed.",
        "",
        "## Reports",
        "",
    ]
    for target, label in MATRIX:
        path = result_path(root, target)
        data = loaded[target]
        lines.append(f"- {label}: `{data.get('overall')}` — `{path}`")
    ready = root / "finalization-ready.md"
    ready.parent.mkdir(parents=True, exist_ok=True)
    ready.write_text("\n".join(lines) + "\n")
    print("\nAll three exact target reports are PASS.")
    print("Runtime metadata changes may now be prepared after reviewing the reports.")
    print("The final package rebuild/install gate remains outstanding; M7 is not marked complete by this script.")
    print(f"Saved: {ready}")
    return 0


def main(argv: list[str]) -> int:
    if argv and argv[0] == "report":
        return report_main(argv[1:])
    if argv and argv[0] == "finalize":
        return finalize_main(argv[1:])
    return acceptance_main(argv)


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except KeyboardInterrupt:
        raise SystemExit(130)
