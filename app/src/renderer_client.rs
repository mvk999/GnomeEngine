use std::{cell::RefCell, ffi::OsStr, path::PathBuf, rc::Rc};

use gio::prelude::*;

const BUS_NAME: &str = "io.github.mvk999.GnomeEngine.Renderer";
const OBJECT_PATH: &str = "/io/github/mvk999/GnomeEngine/Renderer";
const INTERFACE: &str = "io.github.mvk999.GnomeEngine.Renderer";
type ApplyCompletion = Box<dyn FnOnce(Result<(), String>)>;
const INTEGRATION_TIMEOUT_SECONDS: u32 = 8;

struct PendingApply {
    path: String,
    callback: ApplyCompletion,
    timeout: glib::SourceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RendererStatus {
    pub available: bool,
    pub state: String,
    pub current_video: Option<String>,
    pub last_error: Option<String>,
    pub pause_reasons: Vec<String>,
    pub pause_on_battery: bool,
    pub wallpaper_active: bool,
    pub desktop_integration_supported: bool,
    pub desktop_integration_ready: bool,
}

impl Default for RendererStatus {
    fn default() -> Self {
        Self {
            available: false,
            state: "unavailable".to_owned(),
            current_video: None,
            last_error: None,
            pause_reasons: Vec::new(),
            pause_on_battery: true,
            wallpaper_active: false,
            desktop_integration_supported: false,
            desktop_integration_ready: false,
        }
    }
}

#[derive(Default)]
struct ClientInner {
    connection: Option<gio::DBusConnection>,
    subscriptions: Vec<gio::SignalSubscriptionId>,
    status: RendererStatus,
    observers: Vec<Rc<dyn Fn(RendererStatus)>>,
    renderer_process: Option<gio::Subprocess>,
    pending_apply: Option<PendingApply>,
}

#[derive(Clone, Default)]
pub struct RendererClient(Rc<RefCell<ClientInner>>);

impl RendererClient {
    pub fn new() -> Self {
        let client = Self::default();
        let weak = Rc::downgrade(&client.0);
        gio::bus_watch_name(
            gio::BusType::Session,
            BUS_NAME,
            gio::BusNameWatcherFlags::NONE,
            move |connection, _, _| {
                if let Some(inner) = weak.upgrade() {
                    connect_service(&inner, connection);
                }
            },
            {
                let weak = Rc::downgrade(&client.0);
                move |_, _| {
                    if let Some(inner) = weak.upgrade() {
                        disconnect_service(&inner);
                    }
                }
            },
        );
        client
    }

    pub fn status(&self) -> RendererStatus {
        self.0.borrow().status.clone()
    }

    pub fn refresh_status(&self) {
        let Some(connection) = self.0.borrow().connection.clone() else {
            return;
        };
        query_status(&connection, Rc::downgrade(&self.0));
    }

    pub fn connect_status_changed(&self, callback: impl Fn(RendererStatus) + 'static) {
        let callback: Rc<dyn Fn(RendererStatus)> = Rc::new(callback);
        self.0.borrow_mut().observers.push(callback.clone());
        callback(self.status());
    }

    pub fn apply_video(&self, path: String, callback: impl Fn(Result<(), String>) + 'static) {
        let callback: ApplyCompletion = Box::new(callback);
        // End the RefCell borrow before entering either branch. The async GTK
        // callback can re-enter this client while installing pending Apply.
        let existing_connection = { self.0.borrow().connection.clone() };
        if let Some(connection) = existing_connection {
            let status = self.status();
            if !status.desktop_integration_ready {
                if status.available
                    && status.state != "loading-status"
                    && !status.desktop_integration_supported
                {
                    callback(Err(
                        "Renderer is incompatible with desktop integration; restart it before applying".to_owned(),
                    ));
                    return;
                }
                self.queue_apply_until_ready(path, callback);
                query_status(&connection, Rc::downgrade(&self.0));
                return;
            }
            let client = self.clone();
            call_apply(
                connection,
                path,
                Box::new(move |result| {
                    if result.is_ok() {
                        client.refresh_status();
                    }
                    callback(result);
                }),
            );
            return;
        }

        // GetStatus activates the installed D-Bus service without sending
        // ApplyVideo before the Shell extension has prepared the desktop
        // surface. In a development checkout without a D-Bus service file,
        // the renderer executable is started as a local fallback.
        let client = self.clone();
        gio::bus_get(
            gio::BusType::Session,
            None::<&gio::Cancellable>,
            move |connection| match connection {
                Ok(connection) => {
                    client.queue_apply_until_ready(path, callback);
                    let activating_client = client.clone();
                    connection.call(
                        Some(BUS_NAME),
                        OBJECT_PATH,
                        INTERFACE,
                        "GetStatus",
                        None,
                        None,
                        gio::DBusCallFlags::NONE,
                        5000,
                        None::<&gio::Cancellable>,
                        move |result| {
                            if let Err(error) = result {
                                let remote_error = gio::DBusError::remote_error(&error);
                                if is_missing_activation_error(remote_error.as_deref()) {
                                    activating_client.start_renderer_for_apply();
                                } else {
                                    activating_client.fail_pending_apply(format!(
                                        "could not activate renderer: {error}"
                                    ));
                                }
                            }
                        },
                    );
                }
                Err(error) => callback(Err(format!(
                    "Could not connect to the session bus: {error}"
                ))),
            },
        );
    }

    fn queue_apply_until_ready(&self, path: String, callback: ApplyCompletion) {
        let weak = Rc::downgrade(&self.0);
        let timeout = glib::timeout_add_seconds_local_once(
            INTEGRATION_TIMEOUT_SECONDS,
            move || {
                if let Some(inner) = weak.upgrade() {
                    let pending = { inner.borrow_mut().pending_apply.take() };
                    if let Some(pending) = pending {
                        (pending.callback)(Err(
                        "GNOME desktop integration was not confirmed; Apply was canceled to avoid opening a video window".to_owned(),
                    ));
                    }
                }
            },
        );
        let previous = self.0.borrow_mut().pending_apply.replace(PendingApply {
            path,
            callback,
            timeout,
        });
        if let Some(previous) = previous {
            previous.timeout.remove();
            (previous.callback)(Err("A newer Apply request replaced this one".to_owned()));
        }
        if self.status().desktop_integration_ready {
            try_pending_apply(&self.0);
        }
    }

    fn start_renderer_for_apply(&self) {
        if self.0.borrow().pending_apply.is_none() {
            return;
        }
        if let Err(error) = self.start_renderer() {
            self.fail_pending_apply(error);
        }
    }

    fn fail_pending_apply(&self, error: String) {
        fail_pending_apply(&self.0, error);
    }

    pub fn stop(&self, callback: impl Fn(Result<(), String>) + 'static) {
        let client = self.clone();
        self.call("Stop", None, move |result| {
            if result.is_ok() {
                client.refresh_status();
            }
            callback(result.map(|_| ()))
        });
    }

    pub fn pause(&self, callback: impl Fn(Result<(), String>) + 'static) {
        self.call("Pause", None, move |result| callback(result.map(|_| ())));
    }

    pub fn resume(&self, callback: impl Fn(Result<(), String>) + 'static) {
        self.call("Resume", None, move |result| callback(result.map(|_| ())));
    }

    pub fn set_pause_on_battery(
        &self,
        enabled: bool,
        callback: impl Fn(Result<(), String>) + 'static,
    ) {
        self.call(
            "SetPauseOnBattery",
            Some(&(enabled,).to_variant()),
            move |result| callback(result.map(|_| ())),
        );
    }

    fn call(
        &self,
        method: &'static str,
        parameters: Option<&glib::Variant>,
        callback: impl Fn(Result<glib::Variant, String>) + 'static,
    ) {
        let Some(connection) = self.0.borrow().connection.clone() else {
            callback(Err("Renderer service is not running".to_owned()));
            return;
        };
        connection.call(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE,
            method,
            parameters,
            None,
            gio::DBusCallFlags::NONE,
            5000,
            None::<&gio::Cancellable>,
            move |result| callback(result.map_err(|error| error.to_string())),
        );
    }

    fn start_renderer(&self) -> Result<(), String> {
        if self.0.borrow().renderer_process.is_some() {
            return Ok(());
        }
        let executable = renderer_executable().ok_or_else(|| {
            "Renderer is not running and gnomeengine-renderer was not found".to_owned()
        })?;
        let argv: [&OsStr; 1] = [executable.as_os_str()];
        let process = gio::Subprocess::newv(&argv, gio::SubprocessFlags::NONE)
            .map_err(|error| format!("could not start renderer: {error}"))?;
        let waiter = process.clone();
        self.0.borrow_mut().renderer_process = Some(process);
        let weak = Rc::downgrade(&self.0);
        waiter.wait_async(None::<&gio::Cancellable>, move |result| {
            let Some(inner) = weak.upgrade() else {
                return;
            };
            let pending = {
                let mut state = inner.borrow_mut();
                if state.connection.is_some() {
                    return;
                }
                state.renderer_process = None;
                state.pending_apply.take()
            };
            if let Some(pending) = pending {
                pending.timeout.remove();
                let detail = result
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_else(|| "renderer exited before becoming available".to_owned());
                (pending.callback)(Err(format!("Renderer failed to start: {detail}")));
            }
        });
        Ok(())
    }
}

fn renderer_executable() -> Option<PathBuf> {
    let executable_name = if cfg!(windows) {
        "gnomeengine-renderer.exe"
    } else {
        "gnomeengine-renderer"
    };
    let installed = PathBuf::from("/usr/libexec/gnomeengine").join(executable_name);
    if installed.is_file() {
        return Some(installed);
    }
    if let Ok(current) = std::env::current_exe() {
        if let Some(parent) = current.parent() {
            let sibling = parent.join(executable_name);
            if sibling.is_file() {
                return Some(sibling);
            }
        }
    }
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(executable_name))
        .find(|path| path.is_file())
}

fn call_apply(connection: gio::DBusConnection, path: String, callback: ApplyCompletion) {
    call_apply_with_dbus_error(
        connection,
        path,
        Box::new(move |result| callback(result.map_err(|error| error.to_string()))),
    );
}

fn call_apply_with_dbus_error(
    connection: gio::DBusConnection,
    path: String,
    callback: Box<dyn FnOnce(Result<(), glib::Error>)>,
) {
    connection.call(
        Some(BUS_NAME),
        OBJECT_PATH,
        INTERFACE,
        "ApplyVideo",
        Some(&(path,).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        30_000,
        None::<&gio::Cancellable>,
        move |result| callback(result.map(|_| ())),
    );
}

fn is_missing_activation_error(error_name: Option<&str>) -> bool {
    matches!(
        error_name,
        Some(
            "org.freedesktop.DBus.Error.ServiceUnknown"
                | "org.freedesktop.DBus.Error.Spawn.ServiceNotFound"
                | "org.freedesktop.DBus.Error.Spawn.ExecFailed"
                | "org.freedesktop.DBus.Error.Spawn.FileNotFound"
        )
    )
}

fn connect_service(inner: &Rc<RefCell<ClientInner>>, connection: gio::DBusConnection) {
    disconnect_service(inner);
    let mut subscriptions = Vec::new();
    for signal in [
        "StateChanged",
        "PlaybackError",
        "PauseReasonsChanged",
        "PolicyChanged",
        "DesktopIntegrationChanged",
    ] {
        let weak = Rc::downgrade(inner);
        subscriptions.push(connection.signal_subscribe(
            Some(BUS_NAME),
            Some(INTERFACE),
            Some(signal),
            Some(OBJECT_PATH),
            None,
            gio::DBusSignalFlags::NONE,
            move |_, _, _, _, member, parameters| {
                let Some(inner) = weak.upgrade() else {
                    return;
                };
                let mut status = inner.borrow().status.clone();
                match member {
                    "StateChanged" => {
                        if let Some((state,)) = parameters.get::<(String,)>() {
                            status.state = state;
                        }
                    }
                    "PlaybackError" => {
                        if let Some((message,)) = parameters.get::<(String,)>() {
                            status.last_error = Some(message);
                            status.state = "error".to_owned();
                        }
                    }
                    "PauseReasonsChanged" => {
                        if let Some((reasons,)) = parameters.get::<(Vec<String>,)>() {
                            status.pause_reasons = reasons;
                            if status.state != "error" && status.state != "stopped" {
                                status.state = if status.pause_reasons.is_empty() {
                                    "playing".to_owned()
                                } else {
                                    "paused".to_owned()
                                };
                            }
                        }
                    }
                    "PolicyChanged" => {
                        if let Some((policy, enabled)) = parameters.get::<(String, bool)>() {
                            if policy == "pause-on-battery" {
                                status.pause_on_battery = enabled;
                            }
                        }
                    }
                    "DesktopIntegrationChanged" => {
                        if let Some((ready,)) = parameters.get::<(bool,)>() {
                            status.desktop_integration_supported = true;
                            status.desktop_integration_ready = ready;
                        }
                    }
                    _ => {}
                }
                publish(&inner, status);
                if member == "DesktopIntegrationChanged" {
                    try_pending_apply(&inner);
                }
            },
        ));
    }
    {
        let mut state = inner.borrow_mut();
        state.connection = Some(connection.clone());
        state.subscriptions = subscriptions;
        state.renderer_process = None;
        let mut status = state.status.clone();
        status.available = true;
        status.state = "loading-status".to_owned();
        drop(state);
        publish(inner, status);
    }
    query_status(&connection, Rc::downgrade(inner));
}

fn query_status(connection: &gio::DBusConnection, weak: std::rc::Weak<RefCell<ClientInner>>) {
    connection.call(
        Some(BUS_NAME),
        OBJECT_PATH,
        INTERFACE,
        "GetStatus",
        None,
        None,
        gio::DBusCallFlags::NONE,
        5000,
        None::<&gio::Cancellable>,
        move |result| {
            let Some(inner) = weak.upgrade() else {
                return;
            };
            match result {
                Ok(reply) => match parse_status(&reply) {
                    Ok(mut status) => {
                        status.available = true;
                        let integration_supported = status.desktop_integration_supported;
                        publish(&inner, status);
                        if integration_supported {
                            try_pending_apply(&inner);
                        } else {
                            fail_pending_apply(
                                &inner,
                                "Renderer is incompatible with desktop integration; restart it before applying".to_owned(),
                            );
                        }
                    }
                    Err(error) => {
                        eprintln!("gnomeengine: invalid renderer status: {error}");
                        let mut status = inner.borrow().status.clone();
                        status.state = "error".to_owned();
                        status.last_error = Some("Could not read renderer status".to_owned());
                        publish(&inner, status);
                        fail_pending_apply(
                            &inner,
                            "Renderer is incompatible with this version of GnomeEngine; restart the renderer and try again".to_owned(),
                        );
                    }
                },
                Err(error) => {
                    eprintln!("gnomeengine: could not query renderer status: {error}");
                }
            }
        },
    );
}

fn disconnect_service(inner: &Rc<RefCell<ClientInner>>) {
    let (connection, subscriptions, pending) = {
        let mut state = inner.borrow_mut();
        let had_connection = state.connection.is_some();
        (
            state.connection.take(),
            std::mem::take(&mut state.subscriptions),
            if had_connection {
                state.pending_apply.take()
            } else {
                None
            },
        )
    };
    if let Some(connection) = connection {
        for subscription in subscriptions {
            connection.signal_unsubscribe(subscription);
        }
    }
    inner.borrow_mut().renderer_process = None;
    if let Some(pending) = pending {
        pending.timeout.remove();
        (pending.callback)(Err("Renderer service disconnected".to_owned()));
    }
    let status = RendererStatus::default();
    publish(inner, status);
}

fn try_pending_apply(inner: &Rc<RefCell<ClientInner>>) {
    let connection = {
        let state = inner.borrow();
        if !state.status.desktop_integration_ready {
            return;
        }
        state.connection.clone()
    };
    let Some(connection) = connection else {
        return;
    };
    let pending = inner.borrow_mut().pending_apply.take();
    let Some(pending) = pending else {
        return;
    };
    pending.timeout.remove();
    let weak = Rc::downgrade(inner);
    let callback_connection = connection.clone();
    call_apply(
        connection,
        pending.path,
        Box::new(move |result| {
            if result.is_ok() {
                query_status(&callback_connection, weak);
            }
            (pending.callback)(result);
        }),
    );
}

fn fail_pending_apply(inner: &Rc<RefCell<ClientInner>>, error: String) {
    let pending = { inner.borrow_mut().pending_apply.take() };
    if let Some(pending) = pending {
        pending.timeout.remove();
        (pending.callback)(Err(error));
    }
}

fn publish(inner: &Rc<RefCell<ClientInner>>, status: RendererStatus) {
    let observers = {
        let mut state = inner.borrow_mut();
        if state.status == status {
            return;
        }
        state.status = status.clone();
        state.observers.clone()
    };
    for observer in observers {
        observer(status.clone());
    }
}

fn parse_status(reply: &glib::Variant) -> Result<RendererStatus, String> {
    if reply.n_children() != 1 {
        return Err("GetStatus did not return one status dictionary".to_owned());
    }
    let dict = glib::VariantDict::new(Some(&reply.child_value(0)));
    let string = |key: &str| {
        dict.lookup::<String>(key)
            .map_err(|_| format!("GetStatus field {key} has the wrong type"))?
            .ok_or_else(|| format!("GetStatus is missing {key}"))
    };
    let array = |key: &str| {
        dict.lookup::<Vec<String>>(key)
            .map_err(|_| format!("GetStatus field {key} has the wrong type"))?
            .ok_or_else(|| format!("GetStatus is missing {key}"))
    };
    let boolean = |key: &str| {
        dict.lookup::<bool>(key)
            .map_err(|_| format!("GetStatus field {key} has the wrong type"))?
            .ok_or_else(|| format!("GetStatus is missing {key}"))
    };
    Ok(RendererStatus {
        available: true,
        state: string("state")?,
        current_video: optional_string(string("currentVideo")?),
        last_error: optional_string(string("lastError")?),
        pause_reasons: array("pauseReasons")?,
        pause_on_battery: boolean("pauseOnBattery")?,
        wallpaper_active: boolean("wallpaperActive")?,
        desktop_integration_supported: dict
            .lookup::<bool>("desktopIntegrationReady")
            .map_err(|_| "GetStatus field desktopIntegrationReady has the wrong type".to_owned())?
            .is_some(),
        desktop_integration_ready: dict
            .lookup::<bool>("desktopIntegrationReady")
            .map_err(|_| "GetStatus field desktopIntegrationReady has the wrong type".to_owned())?
            .unwrap_or(false),
    })
}

fn optional_string(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{is_missing_activation_error, parse_status};
    use glib::prelude::ToVariant;
    use std::collections::HashMap;

    #[test]
    fn parses_status_dictionary_from_dbus_reply() {
        let fields: HashMap<String, glib::Variant> = HashMap::from([
            ("state".into(), "paused".to_variant()),
            ("currentVideo".into(), "/library/a.mp4".to_variant()),
            ("lastError".into(), "".to_variant()),
            (
                "pauseReasons".into(),
                vec!["on-battery".to_owned()].to_variant(),
            ),
            ("wallpaperActive".into(), true.to_variant()),
            ("desktopIntegrationReady".into(), true.to_variant()),
            ("pauseOnBattery".into(), true.to_variant()),
        ]);
        let reply = glib::Variant::tuple_from_iter([fields.to_variant()]);
        let status = parse_status(&reply).unwrap();
        assert_eq!(status.state, "paused");
        assert_eq!(status.current_video.as_deref(), Some("/library/a.mp4"));
        assert_eq!(status.pause_reasons, ["on-battery"]);
        assert!(status.pause_on_battery);
        assert!(status.wallpaper_active);
        assert!(status.desktop_integration_supported);
        assert!(status.desktop_integration_ready);
        assert!(status.last_error.is_none());
    }

    #[test]
    fn rejects_incomplete_status_dictionary() {
        let reply =
            glib::Variant::tuple_from_iter([HashMap::<String, glib::Variant>::new().to_variant()]);
        assert!(parse_status(&reply).is_err());
    }

    #[test]
    fn identifies_legacy_renderer_status_without_desktop_handshake() {
        let fields: HashMap<String, glib::Variant> = HashMap::from([
            ("state".into(), "playing".to_variant()),
            ("currentVideo".into(), "/library/a.mp4".to_variant()),
            ("lastError".into(), "".to_variant()),
            ("pauseReasons".into(), Vec::<String>::new().to_variant()),
            ("wallpaperActive".into(), true.to_variant()),
            ("pauseOnBattery".into(), true.to_variant()),
        ]);
        let reply = glib::Variant::tuple_from_iter([fields.to_variant()]);
        let status = parse_status(&reply).unwrap();
        assert!(!status.desktop_integration_supported);
        assert!(!status.desktop_integration_ready);
        assert!(status.wallpaper_active);
    }

    #[test]
    fn only_activation_failures_allow_the_development_spawn_fallback() {
        assert!(is_missing_activation_error(Some(
            "org.freedesktop.DBus.Error.ServiceUnknown"
        )));
        assert!(is_missing_activation_error(Some(
            "org.freedesktop.DBus.Error.Spawn.ExecFailed"
        )));
        assert!(!is_missing_activation_error(Some(
            "org.freedesktop.DBus.Error.InvalidArgs"
        )));
        assert!(!is_missing_activation_error(None));
    }
}
