import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Main from 'resource:///org/gnome/shell/ui/main.js';

const RENDERER_NAME = 'io.github.mvk999.GnomeEngine.Renderer';
const RENDERER_PATH = '/io/github/mvk999/GnomeEngine/Renderer';
const RENDERER_INTERFACE = 'io.github.mvk999.GnomeEngine.Renderer';
const EXTENSION_REASONS = ['fullscreen', 'screen-locked', 'display-off'];

export default class GnomeEngineExtension extends Extension {
    enable() {
        this._connections = [];
        this._rendererConnection = null;
        this._lastReasons = new Map();

        this._display = global.display;
        this._monitorManager = global.backend.get_monitor_manager();

        // Connect first, then reconcile from live state to avoid a missed event.
        this._connections.push([
            this._display,
            this._display.connect('in-fullscreen-changed', () => this._syncLifecycle()),
        ]);
        this._connections.push([
            this._monitorManager,
            this._monitorManager.connect('monitors-changed', () => this._syncLifecycle()),
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
                this._syncLifecycle(true);
            },
            () => {
                this._rendererConnection = null;
                this._lastReasons.clear();
            },
        );

        this._syncLifecycle();
    }

    disable() {
        // Keep the lock reason if we are still in unlock-dialog mode: the
        // renderer must not resume behind a locked screen during teardown.
        const locked = this._isLocked();
        for (const reason of EXTENSION_REASONS) {
            this._sendReason(reason, reason === 'screen-locked' && locked);
        }

        if (this._nameWatcher !== undefined) {
            Gio.bus_unwatch_name(this._nameWatcher);
            this._nameWatcher = undefined;
        }
        for (const [object, id] of this._connections ?? []) {
            if (object.signal_handler_is_connected(id))
                object.disconnect(id);
        }
        this._connections = [];
        this._rendererConnection = null;
        this._lastReasons.clear();
        this._display = null;
        this._monitorManager = null;
    }

    _isLocked() {
        return Main.sessionMode.currentMode === 'unlock-dialog';
    }

    _isPrimaryFullscreen() {
        const monitorCount = this._display.get_n_monitors();
        if (monitorCount === 0)
            return false;

        const primary = this._display.get_primary_monitor();
        return this._display.get_monitor_in_fullscreen(primary);
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
