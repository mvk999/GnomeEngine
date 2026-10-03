import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Main from 'resource:///org/gnome/shell/ui/main.js';

const RENDERER_NAME = 'io.github.mvk999.GnomeEngine.Renderer';
const RENDERER_APP_ID = 'io.github.mvk999.GnomeEngine.Renderer';
const RENDERER_PATH = '/io/github/mvk999/GnomeEngine/Renderer';
const RENDERER_INTERFACE = 'io.github.mvk999.GnomeEngine.Renderer';
const EXTENSION_REASONS = ['fullscreen', 'screen-locked', 'display-off'];

export default class GnomeEngineExtension extends Extension {
    enable() {
        this._connections = [];
        this._rendererConnection = null;
        this._rendererWindows = new Map();
        this._lastReasons = new Map();

        this._display = global.display;
        this._monitorManager = global.backend.get_monitor_manager();
        this._bridgeAvailable = typeof Meta.WindowType?.DESKTOP !== 'undefined' &&
            typeof this._display.list_all_windows === 'function' &&
            typeof this._display.get_monitor_geometry === 'function';

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
                for (const window of this._rendererWindows.keys()) {
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
                this._setDesktopIntegrationReady(
                    this._bridgeAvailable,
                    () => this._syncLifecycle(true));
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

        if (this._nameWatcher !== undefined) {
            Gio.bus_unwatch_name(this._nameWatcher);
            this._nameWatcher = undefined;
        }
        for (const [object, id] of this._connections ?? []) {
            if (object.signal_handler_is_connected(id))
                object.disconnect(id);
        }
        for (const [window, id] of this._rendererWindows) {
            if (window.signal_handler_is_connected(id))
                window.disconnect(id);
        }
        this._connections = [];
        this._rendererConnection = null;
        this._lastReasons.clear();
        this._display = null;
        this._monitorManager = null;
        this._rendererWindows.clear();
    }

    _isLocked() {
        return Main.sessionMode.currentMode === 'unlock-dialog';
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
        return window.get_gtk_application_id?.() === RENDERER_APP_ID;
    }

    _maybeConfigureRendererWindow(window) {
        if (!this._isRendererWindow(window) || this._rendererWindows.has(window))
            return;

        const unmanagedId = window.connect('unmanaged', () => {
            this._rendererWindows.delete(window);
            if (window.signal_handler_is_connected(unmanagedId))
                window.disconnect(unmanagedId);
        });
        this._rendererWindows.set(window, unmanagedId);

        try {
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
        const geometry = this._display.get_monitor_geometry(monitor);
        window.move_resize_frame(
            false,
            geometry.x,
            geometry.y,
            geometry.width,
            geometry.height);
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
