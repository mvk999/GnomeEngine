use std::{
    cell::RefCell,
    env,
    error::Error,
    path::{Path, PathBuf},
    rc::Rc,
};

use gst::prelude::*;
use gstreamer as gst;
use gtk::prelude::*;

struct ActivePlayback {
    _bus_watch: gst::bus::BusWatchGuard,
    pipeline: gst::Element,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("gnomeengine-renderer: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: gnomeengine-renderer VIDEO")?;
    if !path.is_file() {
        return Err(format!("not a local file: {}", path.display()).into());
    }

    gst::init()?;
    let app = gtk::Application::builder()
        .application_id("io.github.gnomeengine.Renderer")
        .build();
    let active_playback = Rc::new(RefCell::new(None));
    let playback_for_activate = active_playback.clone();
    app.connect_activate(move |app| match activate_renderer(app, path.as_path()) {
        Ok(playback) => *playback_for_activate.borrow_mut() = Some(playback),
        Err(error) => {
            eprintln!("gnomeengine-renderer: {error}");
            app.quit();
        }
    });
    let playback_on_shutdown = active_playback.clone();
    app.connect_shutdown(move |_| {
        if let Some(playback) = playback_on_shutdown.borrow_mut().take() {
            if let Err(error) = playback.pipeline.set_state(gst::State::Null) {
                eprintln!("gnomeengine-renderer: failed to stop pipeline: {error}");
            }
        }
    });
    app.run_with_args(&["gnomeengine-renderer"]);
    Ok(())
}

fn activate_renderer(
    app: &gtk::Application,
    path: &Path,
) -> Result<ActivePlayback, Box<dyn Error>> {
    let sink = gst::ElementFactory::make("gtk4paintablesink").build()?;
    let paintable = sink.property::<gdk::Paintable>("paintable");
    let pipeline = gst::ElementFactory::make("playbin").build()?;
    pipeline.set_property("uri", gst::glib::filename_to_uri(path, None)?);
    pipeline.set_property("video-sink", &sink);
    pipeline.set_property(
        "audio-sink",
        &gst::ElementFactory::make("fakesink").build()?,
    );

    let picture = gtk::Picture::for_paintable(&paintable);
    picture.set_can_shrink(true);
    picture.set_content_fit(gtk::ContentFit::Cover);
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("GnomeEngine Renderer")
        .default_width(960)
        .default_height(540)
        .child(&picture)
        .build();

    let bus = pipeline.bus().ok_or("GStreamer pipeline has no bus")?;
    let bus_watch = bus.add_watch_local({
        let pipeline = pipeline.clone();
        move |_, message| {
            use gst::MessageView;
            match message.view() {
                MessageView::Eos(..) => {
                    if let Err(error) = pipeline.seek_simple(
                        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT,
                        gst::ClockTime::ZERO,
                    ) {
                        eprintln!("gnomeengine-renderer: loop seek failed: {error}");
                        return glib::ControlFlow::Break;
                    }
                }
                MessageView::Error(error) => {
                    eprintln!("gnomeengine-renderer: {}", error.error());
                    return glib::ControlFlow::Break;
                }
                _ => {}
            }
            glib::ControlFlow::Continue
        }
    })?;

    pipeline.set_state(gst::State::Playing)?;
    eprintln!("INFO renderer initialized; GStreamer autoplugging enabled");
    window.present();
    Ok(ActivePlayback {
        _bus_watch: bus_watch,
        pipeline,
    })
}
