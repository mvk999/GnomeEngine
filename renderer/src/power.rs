use std::{cell::RefCell, rc::Rc};

use gio::prelude::*;

use crate::controller::RendererController;

const UPOWER_NAME: &str = "org.freedesktop.UPower";
const UPOWER_PATH: &str = "/org/freedesktop/UPower";
const UPOWER_DISPLAY_DEVICE_PATH: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const LOGIN_NAME: &str = "org.freedesktop.login1";
const LOGIN_PATH: &str = "/org/freedesktop/login1";

type SubscriptionSet = (gio::DBusConnection, Vec<gio::SignalSubscriptionId>);

#[derive(Clone, Default)]
pub struct SystemObservers(Rc<RefCell<Option<SubscriptionSet>>>);

impl SystemObservers {
    pub fn start(&self, controller: RendererController) {
        let retained = self.0.clone();
        gio::bus_get(
            gio::BusType::System,
            None::<&gio::Cancellable>,
            move |result| {
                let connection = match result {
                    Ok(connection) => connection,
                    Err(error) => {
                        eprintln!(
                            "gnomeengine-renderer: system power monitoring unavailable: {error}"
                        );
                        return;
                    }
                };

                let mut subscriptions = Vec::new();
                observe_upower(&connection, controller.clone(), &mut subscriptions);
                observe_battery_percentage(&connection, controller.clone(), &mut subscriptions);
                observe_logind(&connection, controller, &mut subscriptions);
                *retained.borrow_mut() = Some((connection, subscriptions));
            },
        );
    }

    pub fn stop(&self) {
        if let Some((connection, subscriptions)) = self.0.borrow_mut().take() {
            for subscription in subscriptions {
                connection.signal_unsubscribe(subscription);
            }
        }
    }
}

fn observe_upower(
    connection: &gio::DBusConnection,
    controller: RendererController,
    subscriptions: &mut Vec<gio::SignalSubscriptionId>,
) {
    // Subscribe before taking the initial snapshot. Property events received
    // during the async Get win over that snapshot to avoid a lost transition.
    let initializing = Rc::new(RefCell::new(true));
    let pending_signal = Rc::new(RefCell::new(None::<bool>));
    let init_for_signal = initializing.clone();
    let pending_for_signal = pending_signal.clone();
    let controller_for_signal = controller.clone();
    let subscription = connection.signal_subscribe(
        Some(UPOWER_NAME),
        Some("org.freedesktop.DBus.Properties"),
        Some("PropertiesChanged"),
        Some(UPOWER_PATH),
        Some(UPOWER_NAME),
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, parameters| {
            if let Some((_, changed, _)) = parameters.get::<(
                String,
                std::collections::HashMap<String, glib::Variant>,
                Vec<String>,
            )>() {
                if let Some(value) = changed.get("OnBattery").and_then(|v| v.get::<bool>()) {
                    if *init_for_signal.borrow() {
                        *pending_for_signal.borrow_mut() = Some(value);
                    } else {
                        set_battery_reason(&controller_for_signal, value);
                    }
                }
            }
        },
    );
    subscriptions.push(subscription);

    let controller_for_snapshot = controller;
    connection.call(
        Some(UPOWER_NAME),
        UPOWER_PATH,
        "org.freedesktop.DBus.Properties",
        "Get",
        Some(&(UPOWER_NAME, "OnBattery").to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        3000,
        None::<&gio::Cancellable>,
        move |result| {
            let snapshot = result.ok().and_then(|reply| {
                reply
                    .get::<(glib::Variant,)>()
                    .and_then(|(value,)| value.get::<bool>())
            });
            let final_value = pending_signal.borrow_mut().take().or(snapshot);
            *initializing.borrow_mut() = false;
            if let Some(on_battery) = final_value {
                set_battery_reason(&controller_for_snapshot, on_battery);
            } else {
                eprintln!("gnomeengine-renderer: UPower OnBattery unavailable; battery pause policy disabled");
            }
        },
    );
}

fn set_battery_reason(controller: &RendererController, on_battery: bool) {
    controller.on_battery_changed(on_battery);
}

fn observe_battery_percentage(
    connection: &gio::DBusConnection,
    controller: RendererController,
    subscriptions: &mut Vec<gio::SignalSubscriptionId>,
) {
    let initializing = Rc::new(RefCell::new(true));
    let pending_signal = Rc::new(RefCell::new(None::<Option<f64>>));
    let init_for_signal = initializing.clone();
    let pending_for_signal = pending_signal.clone();
    let controller_for_signal = controller.clone();
    let subscription = connection.signal_subscribe(
        Some(UPOWER_NAME),
        Some("org.freedesktop.DBus.Properties"),
        Some("PropertiesChanged"),
        Some(UPOWER_DISPLAY_DEVICE_PATH),
        Some("org.freedesktop.UPower.Device"),
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, parameters| {
            if let Some((_, changed, invalidated)) = parameters.get::<(
                String,
                std::collections::HashMap<String, glib::Variant>,
                Vec<String>,
            )>() {
                let update = changed.get("Percentage").map(|value| {
                    value
                        .get::<f64>()
                        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
                });
                if let Some(percentage) = update.or_else(|| {
                    invalidated
                        .iter()
                        .any(|property| property == "Percentage")
                        .then_some(None)
                }) {
                    if *init_for_signal.borrow() {
                        *pending_for_signal.borrow_mut() = Some(percentage);
                    } else {
                        controller_for_signal.on_battery_percentage_changed(percentage);
                    }
                }
            }
        },
    );
    subscriptions.push(subscription);

    let controller_for_snapshot = controller;
    connection.call(
        Some(UPOWER_NAME),
        UPOWER_DISPLAY_DEVICE_PATH,
        "org.freedesktop.DBus.Properties",
        "Get",
        Some(&("org.freedesktop.UPower.Device", "Percentage").to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        3000,
        None::<&gio::Cancellable>,
        move |result| {
            let snapshot = result.ok().and_then(|reply| {
                reply
                    .get::<(glib::Variant,)>()
                    .and_then(|(value,)| value.get::<f64>())
                    .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
            });
            let final_value = pending_signal.borrow_mut().take().unwrap_or(snapshot);
            *initializing.borrow_mut() = false;
            if final_value.is_none() {
                eprintln!("gnomeengine-renderer: UPower battery percentage unavailable; low-battery threshold policy will fail open");
            }
            controller_for_snapshot.on_battery_percentage_changed(final_value);
        },
    );
}

fn observe_logind(
    connection: &gio::DBusConnection,
    controller: RendererController,
    subscriptions: &mut Vec<gio::SignalSubscriptionId>,
) {
    let subscription = connection.signal_subscribe(
        Some(LOGIN_NAME),
        Some("org.freedesktop.login1.Manager"),
        Some("PrepareForSleep"),
        Some(LOGIN_PATH),
        None,
        gio::DBusSignalFlags::NONE,
        move |_, _, _, _, _, parameters| {
            if let Some((preparing,)) = parameters.get::<(bool,)>() {
                controller.on_system_sleep_changed(preparing);
            }
        },
    );
    subscriptions.push(subscription);

    connection.call(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
        "NameHasOwner",
        Some(&(LOGIN_NAME,).to_variant()),
        None,
        gio::DBusCallFlags::NONE,
        3000,
        None::<&gio::Cancellable>,
        |result| match result.and_then(|reply| {
            reply
                .get::<(bool,)>()
                .ok_or_else(|| glib::Error::new(gio::DBusError::Failed, "invalid NameHasOwner reply"))
        }) {
            Ok((true,)) => {}
            Ok((false,)) => eprintln!(
                "gnomeengine-renderer: system suspend lifecycle monitoring unavailable (logind is absent)"
            ),
            Err(error) => eprintln!(
                "gnomeengine-renderer: system suspend lifecycle monitoring unavailable: {error}"
            ),
        },
    );
}
