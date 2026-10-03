use crate::{library::Library, renderer_client::RendererClient, window::MainWindow};
use adw::prelude::*;

const APP_ID: &str = "io.github.mvk999.GnomeEngine";

pub fn run() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    let renderer = RendererClient::new();

    app.connect_activate(move |app| {
        if let Some(window) = app.active_window() {
            window.present();
            return;
        }
        let library = match Library::load_default() {
            Ok(library) => library,
            Err(error) => {
                eprintln!("gnomeengine: could not load wallpaper library: {error}");
                Library::empty_default()
            }
        };
        let window = MainWindow::new(app, library, renderer.clone());
        window.present();
    });

    app.set_accels_for_action("app.import", &["<Primary>o"]);
    app.set_accels_for_action("app.preferences", &["<Primary>comma"]);
    app.set_accels_for_action("app.quit", &["<Primary>q"]);
    let quit_action = gio::SimpleAction::new("quit", None);
    quit_action.connect_activate(glib::clone!(
        #[weak]
        app,
        move |_, _| app.quit()
    ));
    app.add_action(&quit_action);

    app.run()
}
