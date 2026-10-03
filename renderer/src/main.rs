use std::{cell::RefCell, env, rc::Rc};

use gstreamer as gst;

mod controller;
mod dbus;
mod desktop_integration;
pub mod lifecycle;
mod power;
mod preferences;

use controller::{RendererController, BUS_NAME};

fn main() {
    if env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("gnomeengine-renderer {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if let Err(error) = run() {
        eprintln!("gnomeengine-renderer: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    gst::init()?;

    let initial_video = env::args_os()
        .nth(1)
        .map(|path| path.to_string_lossy().into_owned());
    let controller = RendererController::new_with_policy_config(preferences::load());
    let system_observers = power::SystemObservers::default();
    system_observers.start(controller.clone());
    let main_loop = glib::MainLoop::new(None, false);
    let registration_id = Rc::new(RefCell::new(None));
    let dbus_connection = Rc::new(RefCell::new(None));

    let loop_on_bus_error = main_loop.clone();
    let registration_on_bus = registration_id.clone();
    let connection_on_bus = dbus_connection.clone();
    let controller_on_bus = controller.clone();
    let loop_on_name_lost = main_loop.clone();
    let controller_on_name_acquired = controller.clone();

    let owner_id = gio::bus_own_name(
        gio::BusType::Session,
        BUS_NAME,
        gio::BusNameOwnerFlags::NONE,
        move |connection, _| {
            controller_on_bus.attach_dbus_connection(connection.clone());
            *connection_on_bus.borrow_mut() = Some(connection.clone());
            match dbus::register_object(&connection, controller_on_bus.clone()) {
                Ok(id) => *registration_on_bus.borrow_mut() = Some(id),
                Err(error) => {
                    eprintln!("gnomeengine-renderer: failed to export D-Bus API: {error}");
                    loop_on_bus_error.quit();
                }
            }
        },
        move |_, _| {
            eprintln!("INFO renderer service acquired session bus name {BUS_NAME}");
            if let Some(path) = initial_video.as_deref() {
                if let Err(error) = controller_on_name_acquired.apply_video(path) {
                    eprintln!("gnomeengine-renderer: {error}");
                }
            }
        },
        move |_, _| {
            eprintln!("gnomeengine-renderer: renderer service name is unavailable");
            loop_on_name_lost.quit();
        },
    );

    main_loop.run();
    system_observers.stop();
    controller.shutdown();
    if let Some(registration_id) = registration_id.borrow_mut().take() {
        if let Some(connection) = dbus_connection.borrow().as_ref() {
            let _ = connection.unregister_object(registration_id);
        }
    }
    gio::bus_unown_name(owner_id);
    Ok(())
}
