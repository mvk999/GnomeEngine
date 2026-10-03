import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const RENDERER_NAME = 'io.github.mvk999.GnomeEngine.Renderer';
const RENDERER_APP_ID = 'io.github.mvk999.GnomeEngine.Renderer';
const RENDERER_PATH = '/io/github/mvk999/GnomeEngine/Renderer';
const RENDERER_INTERFACE = 'io.github.mvk999.GnomeEngine.Renderer';
const INTEGRATION_NAME = 'io.github.mvk999.GnomeEngine.ShellIntegration';
const INTEGRATION_PATH = '/io/github/mvk999/GnomeEngine/ShellIntegration';
const INTEGRATION_INTERFACE = 'io.github.mvk999.GnomeEngine.ShellIntegration';
const INTEGRATION_XML = `<node>
  <interface name="${INTEGRATION_INTERFACE}">
    <method name="EnsureRenderer">
      <arg name="executable" type="s" direction="in"/>
    </method>
  </interface>
</node>`;
const EXTENSION_REASONS = ['fullscreen', 'screen-locked', 'display-off'];

export default class GnomeEngineExtension extends Extension {
    enable() {
        this._connections = [];
        this._rendererConnection = null;
        this._rendererWindows = new Map();
        this._lastReasons = new Map();
        this._legacyWaylandClient = null;
        this._legacyRendererProcess = null;
        this._pendingEnsureInvocations = [];

        this._display = global.display;
        this._monitorManager = global.backend.get_monitor_manager();
        this._bridgeMode = this._selectBridgeMode();
        this._bridgeAvailable = this._bridgeMode !== 'unsupported' &&
            typeof this._display.list_all_windows === 'function' &&
            typeof this._display.get_monitor_geometry === 'function';

        this._integrationNameOwner = Gio.bus_own_name(
            Gio.BusType.SESSION,
            INTEGRATION_NAME,
            Gio.BusNameOwnerFlags.NONE,
            (connection) => this._exportIntegrationApi(connection),
            () => {},
            () => console.error('GnomeEngine: Shell integration D-Bus name is already owned'),
        );

        // Connect first, then reconcile from live state to avoid a missed event.
        this._connections.push([
            this._display,
            this._display.connect('in-fullscreen-changed', () => this._syncLifecycle()),
        ]);
        this._connections.push([
            this._display,
            this._display.connect('window-created', (_display, window) =>
                this._maybeConfigureRendererWindow(window)),
        ]);
        this._connections.push([
            this._monitorManager,
            this._monitorManager.connect('monitors-changed', () => {
                if (this._bridgeMode !== 'legacy-wayland') for (const window of this._rendererWindows.keys()) {
                    try {
                        this._fitRendererWindow(window);
                    } catch (error) {
                        console.warn(`GnomeEngine: could not resize renderer surface: ${error.message}`);
                        this._setDesktopIntegrationReady(false);
                    }
                }
                this._syncLifecycle();
            }),
        ]);
        this._connections.push([
            this._monitorManager,
            this._monitorManager.connect('power-save-mode-changed', () => this._syncLifecycle()),
        ]);
        this._connections.push([
            Main.sessionMode,
            Main.sessionMode.connect('updated', () => this._syncLifecycle()),
        ]);

        this._nameWatcher = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            RENDERER_NAME,
            Gio.BusNameWatcherFlags.NONE,
            (connection, _name, _owner) => {
                this._rendererConnection = connection;
                // ApplyVideo is what creates the renderer surface. Establish
                // readiness before that call; each backend then attaches the
                // newly created surface using its own bridge. On legacy
                // Wayland, only our WaylandClient-owned child is admissible.
                const ready = this._bridgeAvailable &&
                    (this._bridgeMode !== 'legacy-wayland' || this._legacyRendererProcess !== null);
                this._setDesktopIntegrationReady(ready, () => this._syncLifecycle(true));
                for (const window of this._display.list_all_windows())
                    this._maybeConfigureRendererWindow(window);
                this._completeEnsureInvocations();
            },
            () => {
                this._rendererConnection = null;
                this._lastReasons.clear();
            },
        );

        // The renderer may have started before the extension was enabled.
        for (const window of this._display.list_all_windows())
            this._maybeConfigureRendererWindow(window);

        this._syncLifecycle();
    }

    disable() {
        // Keep the lock reason if we are still in unlock-dialog mode: the
        // renderer must not resume behind a locked screen during teardown.
        const locked = this._isLocked();
        for (const reason of EXTENSION_REASONS) {
            this._sendReason(reason, reason === 'screen-locked' && locked);
        }
        this._setDesktopIntegrationReady(false);

        for (const invocation of this._pendingEnsureInvocations)
            invocation.return_dbus_error(INTEGRATION_INTERFACE + '.Disabled', 'GnomeEngine integration was disabled');
        this._pendingEnsureInvocations = [];
        if (this._integrationConnection && this._integrationRegistration)
            this._integrationConnection.unregister_object(this._integrationRegistration);
        this._integrationConnection = null;
        this._integrationRegistration = 0;
        if (this._integrationNameOwner !== undefined) {
            Gio.bus_unown_name(this._integrationNameOwner);
            this._integrationNameOwner = undefined;
        }

        if (this._nameWatcher !== undefined) {
            Gio.bus_unwatch_name(this._nameWatcher);
            this._nameWatcher = undefined;
        }
        for (const [object, id] of this._connections ?? []) {
            try {
                object.disconnect(id);
            } catch (error) {
                console.warn(`GnomeEngine: could not disconnect Shell signal: ${error.message}`);
            }
        }
        for (const [window, id] of this._rendererWindows) {
            try {
                window.disconnect(id);
            } catch (error) {
                console.warn(`GnomeEngine: could not disconnect renderer window signal: ${error.message}`);
            }
        }
        this._connections = [];
        this._rendererConnection = null;
        this._lastReasons.clear();
        if (this._legacyRendererProcess) {
            // GNOME 46 Wayland's owned-client bridge cannot be transferred to
            // a process after the Shell extension relinquishes that client.
            this._legacyRendererProcess.force_exit();
            this._legacyRendererProcess = null;
            this._legacyWaylandClient = null;
        }
        this._display = null;
        this._monitorManager = null;
        this._rendererWindows.clear();
    }

    _isLocked() {
        return Main.sessionMode.currentMode === 'unlock-dialog';
    }

    _selectBridgeMode() {
        const hasModernWindowClassification =
            typeof Meta.Window?.prototype?.set_type === 'function' &&
            typeof Meta.Window?.prototype?.hide_from_window_list === 'function';
        if (hasModernWindowClassification)
            return 'modern-wayland';
        if (Meta.is_wayland_compositor?.())
            return typeof Meta.WaylandClient?.prototype?.spawnv === 'function' &&
                typeof Meta.WaylandClient?.prototype?.owns_window === 'function' &&
                typeof Meta.WaylandClient?.prototype?.make_desktop === 'function'
                ? 'legacy-wayland'
                : 'unsupported';
        // The renderer sets its EWMH desktop/taskbar/workspace hints itself
        // before mapping its native GDK X11 surface.
        return 'x11-ewmh';
    }

    _exportIntegrationApi(connection) {
        this._integrationConnection = connection;
        try {
            const nodeInfo = Gio.DBusNodeInfo.new_for_xml(INTEGRATION_XML);
            this._integrationRegistration = connection.register_object(
                INTEGRATION_PATH,
                nodeInfo.interfaces[0],
                (_connection, _sender, _path, _interface, method, parameters, invocation) => {
                    if (method !== 'EnsureRenderer') {
                        invocation.return_dbus_error(
                            'org.freedesktop.DBus.Error.UnknownMethod',
                            `Unknown integration method: ${method}`);
                        return;
                    }
                    this._ensureRenderer(parameters.deep_unpack()[0], invocation);
                },
                null,
                null,
            );
        } catch (error) {
            console.error(`GnomeEngine: could not export integration API: ${error.message}`);
        }
    }

    _ensureRenderer(executable, invocation) {
        if (this._bridgeMode !== 'legacy-wayland') {
            invocation.return_value(new GLib.Variant('()', []));
            return;
        }
        if (this._legacyRendererProcess) {
            invocation.return_value(new GLib.Variant('()', []));
            return;
        }
        if (this._rendererConnection) {
            invocation.return_dbus_error(
                INTEGRATION_INTERFACE + '.RendererAlreadyRunning',
                'The renderer is already running outside the GNOME 46 Wayland client bridge. Stop and restart it before applying a wallpaper.');
            return;
        }
        if (typeof executable !== 'string' || !GLib.path_is_absolute(executable) ||
            GLib.path_get_basename(executable) !== 'gnomeengine-renderer' ||
            !GLib.file_test(executable, GLib.FileTest.IS_EXECUTABLE)) {
            invocation.return_dbus_error(
                INTEGRATION_INTERFACE + '.InvalidExecutable',
                'GnomeEngine could not locate its renderer executable.');
            return;
        }

        this._pendingEnsureInvocations.push(invocation);
        try {
            const launcher = new Gio.SubprocessLauncher({flags: Gio.SubprocessFlags.NONE});
            let client;
            try {
                // Mutter 14's public GNOME46 constructor takes the launcher.
                // Keep the context overload as a capability fallback for
                // Mutter builds exposing that constructor shape instead.
                client = Meta.WaylandClient.new(launcher);
            } catch (_legacyConstructorError) {
                client = Meta.WaylandClient.new(global.context, launcher);
            }
            const process = client.spawnv(this._display, [executable]);
            this._legacyWaylandClient = client;
            this._legacyRendererProcess = process;
            process.wait_async(null, (child, result) => {
                try {
                    child.wait_finish(result);
                } catch (error) {
                    console.warn(`GnomeEngine: renderer child wait failed: ${error.message}`);
                }
                if (this._legacyRendererProcess !== child)
                    return;
                this._legacyRendererProcess = null;
                this._legacyWaylandClient = null;
                if (!this._rendererConnection)
                    this._failEnsureInvocations('The renderer exited before its D-Bus service became available.');
            });
        } catch (error) {
            this._legacyWaylandClient = null;
            this._legacyRendererProcess = null;
            this._failEnsureInvocations(`Could not start the GNOME 46 Wayland renderer: ${error.message}`);
        }
    }

    _completeEnsureInvocations() {
        for (const invocation of this._pendingEnsureInvocations)
            invocation.return_value(new GLib.Variant('()', []));
        this._pendingEnsureInvocations = [];
    }

    _failEnsureInvocations(message) {
        for (const invocation of this._pendingEnsureInvocations)
            invocation.return_dbus_error(INTEGRATION_INTERFACE + '.StartFailed', message);
        this._pendingEnsureInvocations = [];
    }

    _isPrimaryFullscreen() {
        const monitorCount = this._display.get_n_monitors();
        if (monitorCount === 0)
            return false;

        const primary = this._display.get_primary_monitor();
        return this._display.list_all_windows().some(window =>
            !this._isRendererWindow(window) &&
            window.is_fullscreen() &&
            window.get_monitor() === primary);
    }

    _isRendererWindow(window) {
        if (this._bridgeMode === 'legacy-wayland')
            return this._legacyWaylandClient?.owns_window(window) ?? false;
        return window.get_gtk_application_id?.() === RENDERER_APP_ID ||
            window.get_wm_class?.() === RENDERER_APP_ID ||
            window.get_wm_class_instance?.() === RENDERER_APP_ID;
    }

    _maybeConfigureRendererWindow(window) {
        if (!this._isRendererWindow(window) || this._rendererWindows.has(window))
            return;

        const unmanagedId = window.connect('unmanaged', () => {
            this._rendererWindows.delete(window);
        });
        this._rendererWindows.set(window, unmanagedId);

        try {
            if (this._bridgeMode === 'x11-ewmh') {
                // GDK/Xlib installs the EWMH desktop and no-strut hints before
                // map. Mutter may still initially constrain the desktop window
                // to the workspace work area, so reapply the full monitor
                // rectangle after Shell recognizes the mapped desktop surface.
                this._fitRendererWindow(window);
                this._setDesktopIntegrationReady(true, () => this._syncLifecycle(true));
                console.info('GnomeEngine: renderer surface recognized with X11 EWMH desktop hints');
                return;
            }

            if (this._bridgeMode === 'legacy-wayland') {
                if (!this._legacyWaylandClient?.owns_window(window))
                    throw new Error('GNOME 46 Wayland does not own this renderer surface');
                this._legacyWaylandClient.make_desktop(window);
                this._legacyWaylandClient.hide_from_window_list(window);
                this._logRendererGeometry(window);
                this._setDesktopIntegrationReady(true, () => this._syncLifecycle(true));
                console.info('GnomeEngine: renderer surface attached through the GNOME 46 Wayland client bridge');
                return;
            }

            // This is a Mutter window-type bridge rather than actor reparenting:
            // Mutter keeps ownership of the surface and places DESKTOP windows
            // in its desktop stacking layer beneath normal application windows.
            const requiredMethods = [
                'set_type', 'get_window_type', 'hide_from_window_list',
                'stick', 'move_resize_frame', 'lower',
            ];
            if (typeof Meta.WindowType?.DESKTOP === 'undefined' ||
                requiredMethods.some(method => typeof window[method] !== 'function')) {
                throw new Error('Mutter does not expose the DESKTOP window type');
            }
            window.set_type(Meta.WindowType.DESKTOP);
            if (window.get_window_type() !== Meta.WindowType.DESKTOP)
                throw new Error('Mutter did not accept the DESKTOP window type');
            window.hide_from_window_list();
            window.stick();
            this._fitRendererWindow(window);
            window.lower();
            this._setDesktopIntegrationReady(true, () => this._syncLifecycle(true));
            console.info('GnomeEngine: renderer surface attached as a desktop window');
        } catch (error) {
            console.error(`GnomeEngine: cannot attach renderer surface: ${error.message}`);
            // Fail closed. The renderer stops its GTK surface rather than
            // leaving a player-shaped application window on the desktop.
            this._setDesktopIntegrationReady(false);
            try {
                window.minimize();
            } catch (minimizeError) {
                console.warn(`GnomeEngine: cannot hide failed renderer surface: ${minimizeError.message}`);
            }
        }
    }

    _fitRendererWindow(window) {
        const monitor = this._display.get_primary_monitor();
        if (monitor < 0)
            return;

        // Wallpaper is desktop content, not a normal application window:
        // always size it from the full monitor rectangle. Work area is read
        // below for diagnostics only and must never affect this geometry.
        const geometry = this._display.get_monitor_geometry(monitor);
        window.move_resize_frame(
            false,
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height);

        this._logRendererGeometry(window, geometry);
    }

    _logRendererGeometry(window, monitorGeometry = null) {
        const monitor = this._display.get_primary_monitor();
        if (monitor < 0)
            return;
        const geometry = monitorGeometry ?? this._display.get_monitor_geometry(monitor);
        const rendererGeometry = window.get_frame_rect();
        let workArea = null;
        try {
            const workspace = window.get_workspace();
            if (typeof workspace?.get_work_area_for_monitor === 'function')
                workArea = workspace.get_work_area_for_monitor(monitor);
        } catch (error) {
            console.warn(`GnomeEngine: could not read work area for geometry diagnostics: ${error.message}`);
        }

        const formatGeometry = rect => rect
            ? `${rect.width}x${rect.height}@${rect.x},${rect.y}`
            : 'unavailable';
        console.info(
            `GnomeEngine: wallpaper geometry monitor=${formatGeometry(geometry)} ` +
            `work-area=${formatGeometry(workArea)} renderer=${formatGeometry(rendererGeometry)}`);
    }

    _setDesktopIntegrationReady(ready, callback = null) {
        if (!this._rendererConnection)
            return;

        this._rendererConnection.call(
            RENDERER_NAME,
            RENDERER_PATH,
            RENDERER_INTERFACE,
            'SetDesktopIntegrationReady',
            new GLib.Variant('(b)', [ready]),
            null,
            Gio.DBusCallFlags.NONE,
            2000,
            null,
            (connection, result) => {
                try {
                    connection.call_finish(result);
                    callback?.();
                } catch (error) {
                    console.warn(`GnomeEngine: could not set desktop integration ready=${ready}: ${error.message}`);
                }
            },
        );
    }

    _isDisplayOff() {
        // Mutter exposes this power state for a laptop's built-in panel. It
        // does not expose a public aggregate state for arbitrary external
        // outputs, so unknown/external-only setups remain fail-open.
        if (!this._monitorManager.has_builtin_panel)
            return false;
        return !this._monitorManager.get_is_builtin_display_on();
    }

    _syncLifecycle(force = false) {
        if (!this._display || !this._monitorManager)
            return;

        const desired = new Map([
            ['fullscreen', this._isPrimaryFullscreen()],
            ['screen-locked', this._isLocked()],
            ['display-off', this._isDisplayOff()],
        ]);

        for (const [reason, active] of desired) {
            if (force || this._lastReasons.get(reason) !== active)
                this._sendReason(reason, active);
        }
    }

    _sendReason(reason, active) {
        this._lastReasons.set(reason, active);
        if (!this._rendererConnection)
            return;

        this._rendererConnection.call(
            RENDERER_NAME,
            RENDERER_PATH,
            RENDERER_INTERFACE,
            'SetPauseReason',
            new GLib.Variant('(sb)', [reason, active]),
            null,
            Gio.DBusCallFlags.NONE,
            2000,
            null,
            (connection, result) => {
                try {
                    connection.call_finish(result);
                } catch (error) {
                    console.warn(`GnomeEngine: could not set ${reason}=${active}: ${error.message}`);
                }
            },
        );
    }
}
