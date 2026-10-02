use crate::controller::{RendererController, INTERFACE, OBJECT_PATH};

const INTERFACE_XML: &str = include_str!("../dbus/io.github.mvk999.GnomeEngine.Renderer.xml");

pub fn register_object(
    connection: &gio::DBusConnection,
    controller: RendererController,
) -> Result<gio::RegistrationId, glib::Error> {
    let node_info = gio::DBusNodeInfo::for_xml(INTERFACE_XML)?;
    let interface_info = node_info
        .lookup_interface(INTERFACE)
        .ok_or_else(|| glib::Error::new(gio::DBusError::Failed, "Renderer interface missing"))?;

    connection
        .register_object(OBJECT_PATH, &interface_info)
        .method_call(move |_, _, _, _, method_name, parameters, invocation| {
            dispatch_method(&controller, method_name, &parameters, invocation);
        })
        .build()
}

fn dispatch_method(
    controller: &RendererController,
    method_name: &str,
    parameters: &glib::Variant,
    invocation: gio::DBusMethodInvocation,
) {
    match method_name {
        "ApplyVideo" => match parameters.get::<(String,)>() {
            Some((path,)) => match controller.apply_video(&path) {
                Ok(()) => invocation.return_value(None),
                Err(error) => {
                    invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", &error)
                }
            },
            None => invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.InvalidArgs",
                "ApplyVideo requires a local file path",
            ),
        },
        "Pause" => {
            controller.pause();
            invocation.return_value(None);
        }
        "Resume" => {
            controller.resume();
            invocation.return_value(None);
        }
        "Stop" => {
            controller.stop();
            invocation.return_value(None);
        }
        "GetStatus" => {
            let status = controller.status_variant();
            invocation.return_value(Some(&glib::Variant::tuple_from_iter([status])));
        }
        "SetPauseReason" => match parameters.get::<(String, bool)>() {
            Some((reason, active)) => match controller.set_reason_by_name(&reason, active) {
                Ok(()) => invocation.return_value(None),
                Err(error) => {
                    invocation.return_dbus_error("org.freedesktop.DBus.Error.InvalidArgs", &error)
                }
            },
            None => invocation.return_dbus_error(
                "org.freedesktop.DBus.Error.InvalidArgs",
                "SetPauseReason requires a reason string and active boolean",
            ),
        },
        _ => invocation.return_dbus_error(
            "org.freedesktop.DBus.Error.UnknownMethod",
            "unknown renderer method",
        ),
    }
}
