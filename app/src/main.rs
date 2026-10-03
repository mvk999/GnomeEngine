mod application;
mod library;
mod renderer_client;
mod window;

fn main() -> glib::ExitCode {
    application::run()
}
