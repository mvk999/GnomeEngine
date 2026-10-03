mod application;
mod integration;
mod library;
mod renderer_client;
mod ui;
mod window;

fn main() -> glib::ExitCode {
    if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--version")) {
        println!("gnomeengine {}", env!("CARGO_PKG_VERSION"));
        return glib::ExitCode::SUCCESS;
    }

    application::run()
}
