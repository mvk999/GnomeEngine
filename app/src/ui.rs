use std::path::PathBuf;

use adw::prelude::*;

use crate::{library::Library, renderer_client::RendererStatus};

pub fn install_style() {
    if let Some(display) = gdk::Display::default() {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../resources/style.css"));
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

#[derive(Clone)]
pub struct Sidebar {
    pub widget: gtk::Box,
    pub library_button: gtk::Button,
    pub displays_button: gtk::Button,
    pub settings_button: gtk::Button,
    pub active_title: gtk::Label,
    pub active_state: gtk::Label,
    pub active_caption: gtk::Label,
    active_dot: gtk::Image,
}

pub fn sidebar() -> Sidebar {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_width_request(236);
    outer.set_vexpand(true);
    outer.add_css_class("ge-sidebar");
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_hexpand(true);
    root.set_vexpand(true);
    root.set_margin_top(22);
    root.set_margin_bottom(18);
    root.set_margin_start(14);
    root.set_margin_end(14);
    outer.append(&root);

    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 11);
    brand.set_margin_start(8);
    brand.set_margin_bottom(20);
    let logo_path = logo_path();
    let logo = gtk::Picture::for_filename(&logo_path);
    logo.set_content_fit(gtk::ContentFit::Contain);
    logo.set_size_request(40, 40);
    logo.set_accessible_role(gtk::AccessibleRole::Img);
    logo.update_property(&[gtk::accessible::Property::Label("GnomeEngine")]);
    brand.append(&logo);

    let brand_text = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let name = gtk::Label::new(Some("GnomeEngine"));
    name.set_xalign(0.0);
    name.add_css_class("heading");
    let tagline = gtk::Label::new(Some("Wallpapers, com leveza"));
    tagline.set_xalign(0.0);
    tagline.add_css_class("dim-label");
    tagline.add_css_class("caption");
    brand_text.append(&name);
    brand_text.append(&tagline);
    brand.append(&brand_text);
    root.append(&brand);

    let section = gtk::Label::new(Some("SEU ESPAÇO"));
    section.set_xalign(0.0);
    section.set_margin_start(12);
    section.add_css_class("dim-label");
    section.add_css_class("caption");
    root.append(&section);

    let library_button = navigation_button("view-grid-symbolic", "Biblioteca", true);
    let displays_button = navigation_button("video-display-symbolic", "Telas", false);
    let settings_button = navigation_button("preferences-system-symbolic", "Configurações", false);
    root.append(&library_button);
    root.append(&displays_button);
    root.append(&settings_button);

    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    root.append(&spacer);

    let status_card = gtk::Box::new(gtk::Orientation::Vertical, 5);
    status_card.add_css_class("ge-active-card");
    status_card.set_spacing(5);
    status_card.set_halign(gtk::Align::Fill);
    let status_heading = gtk::Box::new(gtk::Orientation::Horizontal, 7);
    let dot = gtk::Image::from_icon_name("media-record-symbolic");
    dot.set_pixel_size(8);
    dot.set_valign(gtk::Align::Center);
    dot.set_accessible_role(gtk::AccessibleRole::Presentation);
    dot.add_css_class("wallpaper-active-dot");
    dot.set_visible(false);
    let active_state = gtk::Label::new(Some("Nenhum wallpaper ativo"));
    active_state.set_xalign(0.0);
    active_state.set_ellipsize(gtk::pango::EllipsizeMode::End);
    active_state.add_css_class("dim-label");
    active_state.add_css_class("caption");
    status_heading.append(&dot);
    status_heading.append(&active_state);
    status_card.append(&status_heading);
    let active_title = gtk::Label::new(Some(""));
    active_title.set_xalign(0.0);
    active_title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    active_title.add_css_class("heading");
    status_card.append(&active_title);
    let active_caption = gtk::Label::new(Some("Nenhum vídeo em reprodução"));
    active_caption.set_xalign(0.0);
    active_caption.add_css_class("dim-label");
    active_caption.add_css_class("caption");
    status_card.append(&active_caption);
    root.append(&status_card);

    let session = gtk::Box::new(gtk::Orientation::Horizontal, 9);
    session.add_css_class("ge-session-row");
    session.set_margin_top(8);
    let desktop_icon = gtk::Image::from_icon_name("computer-symbolic");
    desktop_icon.set_pixel_size(24);
    desktop_icon.set_valign(gtk::Align::Center);
    desktop_icon.set_accessible_role(gtk::AccessibleRole::Presentation);
    session.append(&desktop_icon);
    let session_details = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let desktop = gtk::Label::new(Some("Meu desktop"));
    desktop.set_xalign(0.0);
    let environment = desktop_environment_label();
    environment.set_xalign(0.0);
    environment.add_css_class("dim-label");
    environment.add_css_class("caption");
    session_details.append(&desktop);
    session_details.append(&environment);
    session.append(&session_details);
    root.append(&session);

    Sidebar {
        widget: outer,
        library_button,
        displays_button,
        settings_button,
        active_title,
        active_state,
        active_caption,
        active_dot: dot,
    }
}

pub fn update_sidebar(sidebar: &Sidebar, status: &RendererStatus, library: &Library) {
    let active = status.wallpaper_active.then_some(()).and_then(|_| {
        status.current_video.as_deref().and_then(|path| {
            library
                .wallpapers()
                .iter()
                .find(|wallpaper| wallpaper.content_path.to_string_lossy().as_ref() == path)
        })
    });
    if let Some(wallpaper) = active {
        sidebar.active_title.set_label(&wallpaper.manifest.title);
        let (state, caption) = match status.state.as_str() {
            "paused" => (
                "Wallpaper pausado".to_owned(),
                status
                    .pause_reasons
                    .first()
                    .map(|reason| format!("Pausado · {}", localized_reason(reason)))
                    .unwrap_or_else(|| "Pausado".to_owned()),
            ),
            "playing" => (
                "Wallpaper ativo".to_owned(),
                if status.desktop_integration_ready {
                    "Em todas as telas".to_owned()
                } else {
                    "Integração GNOME não confirmada".to_owned()
                },
            ),
            "loading" | "loading-status" => {
                ("Wallpaper ativo".to_owned(), "Carregando…".to_owned())
            }
            "error" => (
                "Falha na reprodução".to_owned(),
                "Renderer com erro".to_owned(),
            ),
            _ => (
                "Wallpaper ativo".to_owned(),
                "Renderer conectado".to_owned(),
            ),
        };
        sidebar.active_state.set_label(&state);
        sidebar.active_caption.set_label(&caption);
        if status.state == "paused" {
            sidebar.active_dot.remove_css_class("wallpaper-active-dot");
            sidebar.active_dot.add_css_class("wallpaper-paused-dot");
        } else {
            sidebar.active_dot.remove_css_class("wallpaper-paused-dot");
            sidebar.active_dot.add_css_class("wallpaper-active-dot");
        }
        sidebar.active_dot.set_visible(status.state != "error");
    } else {
        sidebar.active_title.set_label("");
        sidebar.active_state.set_label("Nenhum wallpaper ativo");
        sidebar.active_caption.set_label(if status.available {
            "Pronto para aplicar"
        } else {
            "Renderer indisponível"
        });
        sidebar.active_dot.set_visible(false);
    }
}

fn navigation_button(icon: &str, label: &str, selected: bool) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("flat");
    if selected {
        button.add_css_class("nav-selected");
    }
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.set_margin_start(4);
    content.set_margin_end(4);
    content.set_margin_top(3);
    content.set_margin_bottom(3);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(18);
    image.set_valign(gtk::Align::Center);
    image.set_accessible_role(gtk::AccessibleRole::Presentation);
    content.append(&image);
    let text = gtk::Label::new(Some(label));
    text.set_xalign(0.0);
    content.append(&text);
    button.set_child(Some(&content));
    button.set_halign(gtk::Align::Fill);
    button.set_accessible_role(gtk::AccessibleRole::Button);
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button
}

fn logo_path() -> PathBuf {
    let installed = PathBuf::from("/usr/share/gnomeengine/brand/gnomeengine-logo.svg");
    if installed.is_file() {
        installed
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/brand/gnomeengine-logo.svg")
    }
}

pub(crate) fn update_theme_class(window: &adw::ApplicationWindow) {
    let style_manager = adw::StyleManager::default();
    if style_manager.is_dark() {
        window.remove_css_class("ge-light");
        window.add_css_class("ge-dark");
    } else {
        window.remove_css_class("ge-dark");
        window.add_css_class("ge-light");
    }
}

fn desktop_environment_label() -> gtk::Label {
    gtk::Label::new(Some(&desktop_session_description()))
}

pub(crate) fn desktop_session_description() -> String {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "GNOME".to_owned());
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_owned());
    let desktop_name = desktop
        .split(':')
        .find(|name| name.eq_ignore_ascii_case("gnome"))
        .unwrap_or_else(|| desktop.split(':').next().unwrap_or("GNOME"));
    format!(
        "{} · {}",
        desktop_name.to_uppercase(),
        match session.to_ascii_lowercase().as_str() {
            "x11" => "X11",
            "wayland" => "Wayland",
            _ => session.as_str(),
        }
    )
}

pub(crate) fn localized_reason(reason: &str) -> &'static str {
    match reason {
        "manual" => "pausa manual",
        "fullscreen" => "tela cheia",
        "screen-locked" => "tela bloqueada",
        "display-off" => "tela desligada",
        "on-battery" => "bateria",
        "system-sleep" => "suspensão",
        _ => "outro motivo",
    }
}
