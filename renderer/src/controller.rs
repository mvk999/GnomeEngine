use std::{cell::RefCell, path::Path, rc::Rc};

use gio::prelude::*;
use gst::prelude::*;
use gstreamer as gst;
use gtk::prelude::*;

use crate::lifecycle::{LifecyclePolicy, LifecyclePolicyConfig, PauseReason};

pub const BUS_NAME: &str = "io.github.mvk999.GnomeEngine.Renderer";
pub const OBJECT_PATH: &str = "/io/github/mvk999/GnomeEngine/Renderer";
pub const INTERFACE: &str = "io.github.mvk999.GnomeEngine.Renderer";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RendererState {
    Stopped,
    Loading,
    Playing,
    Paused,
    Error,
}

impl RendererState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Loading => "loading",
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Error => "error",
        }
    }
}

struct ActivePlayback {
    _bus_watch: gst::bus::BusWatchGuard,
    pipeline: gst::Element,
    window: gtk::Window,
}

struct ControllerInner {
    active: Option<ActivePlayback>,
    state: RendererState,
    current_video: Option<String>,
    last_error: Option<String>,
    lifecycle: LifecyclePolicy,
    dbus_connection: Option<gio::DBusConnection>,
}

#[derive(Clone)]
pub struct RendererController(Rc<RefCell<ControllerInner>>);

impl RendererController {
    #[cfg(test)]
    pub fn new() -> Self {
        Self::new_with_policy_config(LifecyclePolicyConfig::default())
    }

    pub fn new_with_policy_config(config: LifecyclePolicyConfig) -> Self {
        Self(Rc::new(RefCell::new(ControllerInner {
            active: None,
            state: RendererState::Stopped,
            current_video: None,
            last_error: None,
            lifecycle: LifecyclePolicy::with_config(config),
            dbus_connection: None,
        })))
    }

    pub fn attach_dbus_connection(&self, connection: gio::DBusConnection) {
        self.0.borrow_mut().dbus_connection = Some(connection);
    }

    pub fn reason_names(&self) -> Vec<String> {
        self.0
            .borrow()
            .lifecycle
            .reason_names()
            .map(str::to_owned)
            .collect()
    }

    pub fn status_variant(&self) -> glib::Variant {
        let inner = self.0.borrow();
        let status = glib::VariantDict::new(None);
        let reasons = inner
            .lifecycle
            .reason_names()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        status.insert("state", inner.state.as_str());
        status.insert(
            "currentVideo",
            inner.current_video.as_deref().unwrap_or_default(),
        );
        status.insert("lastError", inner.last_error.as_deref().unwrap_or_default());
        status.insert("pauseReasons", &reasons);
        status.insert("wallpaperActive", inner.active.is_some());
        status.insert("pauseOnBattery", inner.lifecycle.pause_on_battery());
        status.end()
    }

    pub fn pause(&self) {
        self.set_reason(PauseReason::Manual, true);
    }

    pub fn resume(&self) {
        self.set_reason(PauseReason::Manual, false);
    }

    pub fn on_battery_changed(&self, on_battery: bool) {
        let changed = self.0.borrow_mut().lifecycle.on_battery_changed(on_battery);
        self.after_reason_change(changed);
    }

    pub fn set_pause_on_battery(&self, enabled: bool) -> Result<(), String> {
        if self.0.borrow().lifecycle.pause_on_battery() == enabled {
            return Ok(());
        }
        crate::preferences::save_pause_on_battery(enabled)?;
        let (changed, policy_changed) = {
            let mut inner = self.0.borrow_mut();
            let was_enabled = inner.lifecycle.pause_on_battery();
            let changed = inner.lifecycle.set_pause_on_battery(enabled);
            (changed, was_enabled != enabled)
        };
        if changed {
            self.after_reason_change(true);
        }
        if policy_changed {
            self.emit_signal("PolicyChanged", &("pause-on-battery", enabled).to_variant());
        }
        Ok(())
    }

    pub fn on_system_sleep_changed(&self, preparing_for_sleep: bool) {
        self.set_reason(PauseReason::SystemSleep, preparing_for_sleep);
    }

    pub fn set_reason_by_name(&self, name: &str, active: bool) -> Result<(), String> {
        let changed = self
            .0
            .borrow_mut()
            .lifecycle
            .set_reason_by_name(name, active)?;
        self.after_reason_change(changed);
        Ok(())
    }

    fn set_reason(&self, reason: PauseReason, active: bool) {
        let changed = self.0.borrow_mut().lifecycle.set_reason(reason, active);
        self.after_reason_change(changed);
    }

    fn after_reason_change(&self, changed: bool) {
        if !changed {
            return;
        }

        let reasons = self.reason_names();
        self.emit_signal("PauseReasonsChanged", &(reasons,).to_variant());
        self.reconcile_playback();
    }

    pub fn apply_video(&self, file_path: &str) -> Result<(), String> {
        let path = Path::new(file_path);
        let metadata = std::fs::metadata(path)
            .map_err(|error| format!("cannot access video path: {error}"))?;
        if !metadata.file_type().is_file() {
            return Err("video path must be a regular local file".to_owned());
        }
        let path = path
            .canonicalize()
            .map_err(|error| format!("cannot resolve video path: {error}"))?;

        gtk::init().map_err(|error| format!("GTK initialization failed: {error}"))?;
        let (pipeline, paintable) = Self::build_pipeline(&path)?;
        let picture = gtk::Picture::for_paintable(&paintable);
        picture.set_can_shrink(true);
        picture.set_content_fit(gtk::ContentFit::Cover);
        let window = gtk::Window::builder()
            .title("GnomeEngine Renderer")
            .default_width(960)
            .default_height(540)
            .child(&picture)
            .build();

        let bus = pipeline
            .bus()
            .ok_or_else(|| "GStreamer pipeline has no bus".to_owned())?;
        let weak_controller = Rc::downgrade(&self.0);
        let watched_pipeline = pipeline.clone();
        let bus_watch = bus
            .add_watch_local(move |_, message| {
                use gst::MessageView;
                match message.view() {
                    MessageView::Eos(..) => {
                        if let Err(error) = watched_pipeline.seek_simple(
                            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                            gst::ClockTime::ZERO,
                        ) {
                            if let Some(inner) = weak_controller.upgrade() {
                                RendererController(inner).playback_error(error.to_string());
                            }
                            return glib::ControlFlow::Break;
                        }
                    }
                    MessageView::Error(error) => {
                        if let Some(inner) = weak_controller.upgrade() {
                            RendererController(inner).playback_error(error.error().to_string());
                        }
                        return glib::ControlFlow::Break;
                    }
                    _ => {}
                }
                glib::ControlFlow::Continue
            })
            .map_err(|error| format!("cannot watch GStreamer bus: {error}"))?;

        self.stop_active_pipeline();
        let video_name = path.to_string_lossy().into_owned();
        {
            let mut inner = self.0.borrow_mut();
            inner.current_video = Some(video_name);
            inner.last_error = None;
            inner.active = Some(ActivePlayback {
                _bus_watch: bus_watch,
                pipeline: pipeline.clone(),
                window: window.clone(),
            });
        }
        self.set_state(RendererState::Loading);
        window.present();
        self.reconcile_playback();
        eprintln!("INFO renderer initialized; GStreamer autoplugging enabled");
        Ok(())
    }

    fn build_pipeline(path: &Path) -> Result<(gst::Element, gdk::Paintable), String> {
        let sink = gst::ElementFactory::make("gtk4paintablesink")
            .build()
            .map_err(|error| format!("cannot create GTK GStreamer sink: {error}"))?;
        let paintable = sink.property::<gdk::Paintable>("paintable");
        let pipeline = gst::ElementFactory::make("playbin")
            .build()
            .map_err(|error| format!("cannot create GStreamer playbin: {error}"))?;
        let uri = gst::glib::filename_to_uri(path, None)
            .map_err(|error| format!("cannot convert video path to URI: {error}"))?;
        pipeline.set_property("uri", uri);
        pipeline.set_property("video-sink", &sink);
        let audio_sink = gst::ElementFactory::make("fakesink")
            .build()
            .map_err(|error| format!("cannot create audio fakesink: {error}"))?;
        pipeline.set_property("audio-sink", &audio_sink);
        Ok((pipeline, paintable))
    }

    pub fn stop(&self) {
        let manual_changed = self
            .0
            .borrow_mut()
            .lifecycle
            .set_reason(PauseReason::Manual, false);
        self.stop_active_pipeline();
        {
            let mut inner = self.0.borrow_mut();
            inner.current_video = None;
            inner.last_error = None;
        }
        self.set_state(RendererState::Stopped);
        if manual_changed {
            let reasons = self.reason_names();
            self.emit_signal("PauseReasonsChanged", &(reasons,).to_variant());
        }
    }

    pub fn shutdown(&self) {
        self.stop_active_pipeline();
    }

    fn stop_active_pipeline(&self) {
        let active = self.0.borrow_mut().active.take();
        if let Some(active) = active {
            if let Err(error) = active.pipeline.set_state(gst::State::Null) {
                eprintln!("gnomeengine-renderer: failed to stop pipeline: {error}");
            }
            active.window.close();
            drop(active);
        }
    }

    fn reconcile_playback(&self) {
        let (pipeline, paused) = {
            let inner = self.0.borrow();
            if inner.state == RendererState::Error {
                return;
            }
            let Some(active) = inner.active.as_ref() else {
                return;
            };
            (active.pipeline.clone(), inner.lifecycle.is_paused())
        };

        let target = if paused {
            gst::State::Paused
        } else {
            gst::State::Playing
        };
        if let Err(error) = pipeline.set_state(target) {
            self.playback_error(format!("cannot change GStreamer state: {error}"));
            return;
        }
        self.set_state(if paused {
            RendererState::Paused
        } else {
            RendererState::Playing
        });
    }

    fn playback_error(&self, error: String) {
        self.stop_active_pipeline();
        self.0.borrow_mut().last_error = Some(error.clone());
        self.set_state(RendererState::Error);
        self.emit_signal("PlaybackError", &(error,).to_variant());
    }

    fn set_state(&self, state: RendererState) {
        let changed = {
            let mut inner = self.0.borrow_mut();
            if inner.state == state {
                false
            } else {
                inner.state = state;
                true
            }
        };
        if changed {
            self.emit_signal("StateChanged", &(state.as_str(),).to_variant());
        }
    }

    fn emit_signal(&self, name: &str, parameters: &glib::Variant) {
        let connection = self.0.borrow().dbus_connection.clone();
        if let Some(connection) = connection {
            if let Err(error) =
                connection.emit_signal(None, OBJECT_PATH, INTERFACE, name, Some(parameters))
            {
                eprintln!("gnomeengine-renderer: failed to emit {name}: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RendererController, RendererState};

    #[test]
    fn resume_removes_only_manual_reason() {
        let controller = RendererController::new();
        controller.pause();
        controller.set_reason_by_name("fullscreen", true).unwrap();

        controller.resume();

        assert_eq!(controller.reason_names(), ["fullscreen"]);
        assert_eq!(
            glib::VariantDict::new(Some(&controller.status_variant()))
                .lookup::<String>("state")
                .unwrap(),
            Some("stopped".to_owned())
        );
    }

    #[test]
    fn stop_during_automatic_pause_stays_stopped_after_reason_clears() {
        let controller = RendererController::new();
        controller.set_reason_by_name("fullscreen", true).unwrap();

        controller.stop();
        controller.set_reason_by_name("fullscreen", false).unwrap();

        assert_eq!(
            glib::VariantDict::new(Some(&controller.status_variant()))
                .lookup::<String>("state")
                .unwrap(),
            Some(RendererState::Stopped.as_str().to_owned())
        );
        assert!(controller.reason_names().is_empty());
    }

    #[test]
    fn status_exposes_active_pause_reasons() {
        let controller = RendererController::new();
        controller.set_reason_by_name("on-battery", true).unwrap();
        controller.set_reason_by_name("system-sleep", true).unwrap();

        assert_eq!(
            glib::VariantDict::new(Some(&controller.status_variant()))
                .lookup::<Vec<String>>("pauseReasons")
                .unwrap(),
            Some(vec!["on-battery".to_owned(), "system-sleep".to_owned()])
        );
    }

    #[test]
    fn battery_and_suspend_reasons_clear_independently() {
        let controller = RendererController::new();
        controller.on_battery_changed(true);
        controller.on_system_sleep_changed(true);
        controller.on_system_sleep_changed(false);

        assert_eq!(controller.reason_names(), ["on-battery"]);
        controller.on_battery_changed(false);
        assert!(controller.reason_names().is_empty());
    }

    #[test]
    fn fullscreen_and_battery_reasons_do_not_cancel_each_other() {
        let controller = RendererController::new();
        controller.set_reason_by_name("fullscreen", true).unwrap();
        controller.on_battery_changed(true);
        controller.set_reason_by_name("fullscreen", false).unwrap();

        assert_eq!(controller.reason_names(), ["on-battery"]);
        controller.on_battery_changed(false);
        assert!(controller.reason_names().is_empty());
    }

    #[test]
    fn removing_battery_reason_does_not_override_lock() {
        let controller = RendererController::new();
        controller.on_battery_changed(true);
        controller
            .set_reason_by_name("screen-locked", true)
            .unwrap();
        controller.on_battery_changed(false);

        assert_eq!(controller.reason_names(), ["screen-locked"]);
        controller
            .set_reason_by_name("screen-locked", false)
            .unwrap();
        assert!(controller.reason_names().is_empty());
    }

    #[test]
    fn battery_policy_can_be_disabled_without_affecting_sleep_policy() {
        let controller =
            RendererController::new_with_policy_config(crate::lifecycle::LifecyclePolicyConfig {
                pause_on_battery: false,
            });

        controller.on_battery_changed(true);
        assert!(controller.reason_names().is_empty());
        controller.on_system_sleep_changed(true);
        assert_eq!(controller.reason_names(), ["system-sleep"]);
    }
}
