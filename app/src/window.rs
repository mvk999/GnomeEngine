use std::{cell::RefCell, path::PathBuf, rc::Rc, thread};

use adw::prelude::*;

use crate::library::{import_video, Library, Wallpaper};
use crate::renderer_client::{RendererClient, RendererStatus};
use crate::ui::{self, Sidebar};

type OpenHandler = Rc<dyn Fn(Wallpaper)>;
type OpenHandlerSlot = Rc<RefCell<Option<OpenHandler>>>;

struct DetailContext {
    parent: adw::ApplicationWindow,
    stack: gtk::Stack,
    detail: gtk::Box,
    header_title: gtk::Label,
    back_button: gtk::Button,
    active_preview: Rc<RefCell<Option<gtk::Video>>>,
    renderer: RendererClient,
    toast_overlay: adw::ToastOverlay,
    library: Rc<RefCell<Library>>,
    open_handler_slot: OpenHandlerSlot,
    active_path: Rc<RefCell<Option<String>>>,
    detail_controls: Rc<RefCell<Option<DetailControls>>>,
}

#[derive(Clone)]
struct DetailControls {
    path: String,
    apply: gtk::Button,
    stop: gtk::Button,
    status: gtk::Label,
}

struct RemovalContext {
    library: Rc<RefCell<Library>>,
    stack: gtk::Stack,
    toast_overlay: adw::ToastOverlay,
    header_title: gtk::Label,
    back_button: gtk::Button,
    open_handler_slot: OpenHandlerSlot,
    active_path: Rc<RefCell<Option<String>>>,
}

struct ImportContext {
    parent: adw::ApplicationWindow,
    library: Rc<RefCell<Library>>,
    stack: gtk::Stack,
    toast_overlay: adw::ToastOverlay,
    open_handler_slot: OpenHandlerSlot,
    active_path: Rc<RefCell<Option<String>>>,
}

pub struct MainWindow {
    window: adw::ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application, library: Library, renderer: RendererClient) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("GnomeEngine")
            .default_width(1160)
            .default_height(760)
            .width_request(760)
            .height_request(540)
            .build();

        let toolbar = adw::ToolbarView::new();
        let toast_overlay = adw::ToastOverlay::new();
        let header = adw::HeaderBar::new();
        let title_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let header_title = gtk::Label::new(Some("Biblioteca"));
        header_title.add_css_class("title");
        let renderer_status_label = gtk::Label::new(Some("Conectando ao renderer…"));
        renderer_status_label.add_css_class("dim-label");
        renderer_status_label.add_css_class("caption");
        title_box.append(&header_title);
        title_box.append(&renderer_status_label);
        header.set_title_widget(Some(&title_box));

        let stop_button = gtk::Button::from_icon_name("media-playback-stop-symbolic");
        stop_button.set_tooltip_text(Some("Parar wallpaper"));
        stop_button.update_property(&[gtk::accessible::Property::Label("Parar wallpaper")]);
        stop_button.set_visible(false);
        header.pack_end(&stop_button);
        let manual_pause_button = gtk::Button::from_icon_name("media-playback-pause-symbolic");
        manual_pause_button.set_tooltip_text(Some("Pausar wallpaper"));
        manual_pause_button
            .update_property(&[gtk::accessible::Property::Label("Pausar wallpaper")]);
        manual_pause_button.set_visible(false);
        header.pack_end(&manual_pause_button);

        let menu = gio::Menu::new();
        menu.append(Some("Configurações"), Some("app.preferences"));
        menu.append(Some("Sobre o GnomeEngine"), Some("app.about"));
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menu principal")
            .menu_model(&menu)
            .build();
        menu_button.update_property(&[gtk::accessible::Property::Label("Menu principal")]);
        header.pack_end(&menu_button);
        let import_button = gtk::Button::from_icon_name("list-add-symbolic");
        import_button.set_tooltip_text(Some("Importar wallpaper"));
        import_button.update_property(&[gtk::accessible::Property::Label("Importar wallpaper")]);
        import_button.set_label("Importar wallpaper");
        import_button.add_css_class("suggested-action");
        import_button.set_action_name(Some("app.import"));
        header.pack_end(&import_button);
        toolbar.add_top_bar(&header);

        let library = Rc::new(RefCell::new(library));
        if library.borrow().invalid_items() > 0 {
            toast_overlay.add_toast(adw::Toast::new(
                "Alguns itens da biblioteca não puderam ser carregados",
            ));
        }
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::SlideLeftRight)
            .build();
        let active_preview = Rc::new(RefCell::new(None::<gtk::Video>));
        let active_path = Rc::new(RefCell::new(if renderer.status().wallpaper_active {
            renderer.status().current_video.clone()
        } else {
            None
        }));

        {
            let renderer = renderer.clone();
            let toast_overlay = toast_overlay.clone();
            stop_button.connect_clicked(move |_| {
                renderer.stop({
                    let toast_overlay = toast_overlay.clone();
                    move |result| match result {
                        Ok(()) => toast_overlay.add_toast(adw::Toast::new("Wallpaper parado")),
                        Err(error) => {
                            eprintln!("gnomeengine: could not stop wallpaper: {error}");
                            toast_overlay
                                .add_toast(adw::Toast::new("Não foi possível parar o wallpaper"));
                        }
                    }
                });
            });
        }

        {
            let renderer = renderer.clone();
            manual_pause_button.connect_clicked(move |button| {
                let manual_paused = renderer
                    .status()
                    .pause_reasons
                    .iter()
                    .any(|reason| reason == "manual");
                button.set_sensitive(false);
                if manual_paused {
                    renderer.resume(playback_control_result(button.clone(), renderer.clone()));
                } else {
                    renderer.pause(playback_control_result(button.clone(), renderer.clone()));
                }
            });
        }

        let detail_view = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        detail_view.set_margin_top(24);
        detail_view.set_margin_bottom(24);
        detail_view.set_margin_start(24);
        detail_view.set_margin_end(24);
        if let Ok(condition) = adw::BreakpointCondition::parse("max-width: 900px") {
            let breakpoint = adw::Breakpoint::new(condition);
            breakpoint.add_setter(
                &detail_view,
                "orientation",
                Some(&gtk::Orientation::Vertical.to_value()),
            );
            window.add_breakpoint(breakpoint);
        }
        stack.add_named(&detail_view, Some("detail"));
        let detail_controls = Rc::new(RefCell::new(None::<DetailControls>));
        let back_button = gtk::Button::from_icon_name("go-previous-symbolic");
        back_button.set_tooltip_text(Some("Voltar à biblioteca"));
        back_button.update_property(&[gtk::accessible::Property::Label("Voltar à biblioteca")]);
        back_button.set_visible(false);
        header.pack_start(&back_button);
        let open_handler_slot = Rc::new(RefCell::new(None::<OpenHandler>));
        let open_handler: OpenHandler = {
            let stack = stack.clone();
            let detail_view = detail_view.clone();
            let header_title = header_title.clone();
            let back_button = back_button.clone();
            let active_preview = active_preview.clone();
            let renderer = renderer.clone();
            let toast_overlay = toast_overlay.clone();
            let library = library.clone();
            let open_handler_slot = Rc::downgrade(&open_handler_slot);
            let active_path = active_path.clone();
            let window = window.clone();
            let detail_controls = detail_controls.clone();
            Rc::new(move |wallpaper| {
                let Some(open_handler_slot) = open_handler_slot.upgrade() else {
                    return;
                };
                show_detail(
                    DetailContext {
                        parent: window.clone(),
                        stack: stack.clone(),
                        detail: detail_view.clone(),
                        header_title: header_title.clone(),
                        back_button: back_button.clone(),
                        active_preview: active_preview.clone(),
                        renderer: renderer.clone(),
                        toast_overlay: toast_overlay.clone(),
                        library: library.clone(),
                        open_handler_slot,
                        active_path: active_path.clone(),
                        detail_controls: detail_controls.clone(),
                    },
                    &wallpaper,
                );
            })
        };
        *open_handler_slot.borrow_mut() = Some(open_handler.clone());
        stack.add_named(
            &library_view(
                &library.borrow(),
                open_handler.clone(),
                active_path.borrow().as_deref(),
            ),
            Some("library"),
        );
        // GtkStack displays its first child by default. The detail container is
        // added above as a reusable destination, so explicitly select Library
        // before presenting the window (otherwise users see a blank page).
        stack.set_visible_child_name("library");
        let sidebar = ui::sidebar();
        let displays_view = displays_page(&renderer);
        let settings_view = settings_page(&renderer, &toast_overlay);
        stack.add_named(&displays_view, Some("displays"));
        stack.add_named(&settings_view, Some("settings"));
        toolbar.set_content(Some(&stack));

        let sidebar_page = adw::NavigationPage::new(&sidebar.widget, "Navegação");
        let content_page = adw::NavigationPage::new(&toolbar, "GnomeEngine");
        let navigation = adw::NavigationSplitView::new();
        navigation.set_sidebar(Some(&sidebar_page));
        navigation.set_content(Some(&content_page));
        navigation.set_min_sidebar_width(220.0);
        navigation.set_max_sidebar_width(260.0);
        navigation.set_sidebar_width_fraction(0.22);
        toast_overlay.set_child(Some(&navigation));
        window.set_content(Some(&toast_overlay));

        connect_navigation(
            &sidebar,
            &stack,
            &header_title,
            &back_button,
            &active_preview,
            &detail_view,
            &detail_controls,
        );
        ui::update_sidebar(&sidebar, &renderer.status(), &library.borrow());

        {
            let renderer_status_label = renderer_status_label.clone();
            let stop_button = stop_button.clone();
            let manual_pause_button = manual_pause_button.clone();
            let stack = stack.clone();
            let library = library.clone();
            let open_handler_slot = open_handler_slot.clone();
            let active_path = active_path.clone();
            let sidebar = sidebar.clone();
            let detail_controls = detail_controls.clone();
            renderer.connect_status_changed(move |status| {
                update_renderer_status(
                    &renderer_status_label,
                    &stop_button,
                    &manual_pause_button,
                    &status,
                );
                let new_path = if status.wallpaper_active {
                    status.current_video.clone()
                } else {
                    None
                };
                if *active_path.borrow() != new_path {
                    *active_path.borrow_mut() = new_path;
                    if stack.visible_child_name().as_deref() == Some("library") {
                        refresh_library_view(
                            &stack,
                            &library,
                            &open_handler_slot,
                            active_path.borrow().as_deref(),
                        );
                    }
                }
                ui::update_sidebar(&sidebar, &status, &library.borrow());
                update_detail_controls(&detail_controls, &status);
            });
        }

        {
            let stack = stack.clone();
            let header_title = header_title.clone();
            let back_button = back_button.clone();
            let active_preview = active_preview.clone();
            let detail_view = detail_view.clone();
            let library = library.clone();
            let open_handler_slot = open_handler_slot.clone();
            let active_path = active_path.clone();
            let button_for_callback = back_button.clone();
            let detail_controls = detail_controls.clone();
            back_button.connect_clicked(move |_| {
                stop_preview(&active_preview, &detail_view);
                detail_controls.borrow_mut().take();
                stack.set_visible_child_name("library");
                refresh_library_view(
                    &stack,
                    &library,
                    &open_handler_slot,
                    active_path.borrow().as_deref(),
                );
                header_title.set_label("Biblioteca");
                button_for_callback.set_visible(false);
            });
        }

        let action = gio::SimpleAction::new("import", None);
        action.connect_activate({
            let window = window.clone();
            let library = library.clone();
            let stack = stack.clone();
            let toast_overlay = toast_overlay.clone();
            let active_preview = active_preview.clone();
            let detail_view = detail_view.clone();
            let header_title = header_title.clone();
            let back_button = back_button.clone();
            let active_path = active_path.clone();
            let sidebar = sidebar.clone();
            let detail_controls = detail_controls.clone();
            let open_handler_slot = open_handler_slot.clone();
            move |_, _| {
                stop_preview(&active_preview, &detail_view);
                detail_controls.borrow_mut().take();
                stack.set_visible_child_name("library");
                header_title.set_label("Biblioteca");
                back_button.set_visible(false);
                set_sidebar_selection(&sidebar, "library");

                let dialog = gtk::FileDialog::builder()
                    .title("Importar wallpaper")
                    .build();
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Arquivos de vídeo"));
                filter.add_mime_type("video/*");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));
                dialog.set_default_filter(Some(&filter));

                dialog.open(Some(&window), gio::Cancellable::NONE, {
                    let library = library.clone();
                    let stack = stack.clone();
                    let toast_overlay = toast_overlay.clone();
                    let open_handler_slot = open_handler_slot.clone();
                    let active_path = active_path.clone();
                    let parent = window.clone();
                    move |result| {
                        let file = match result {
                            Ok(file) => file,
                            Err(error) if error.matches(gio::IOErrorEnum::Cancelled) => return,
                            Err(error) => {
                                eprintln!("gnomeengine: file chooser failed: {error}");
                                toast_overlay.add_toast(adw::Toast::new(
                                    "Não foi possível abrir o seletor de arquivos",
                                ));
                                return;
                            }
                        };
                        let Some(source) = file.path() else {
                            toast_overlay
                                .add_toast(adw::Toast::new("Escolha um arquivo de vídeo local"));
                            return;
                        };
                        start_import(
                            source,
                            library.borrow().root().to_path_buf(),
                            ImportContext {
                                parent: parent.clone(),
                                library: library.clone(),
                                stack: stack.clone(),
                                toast_overlay: toast_overlay.clone(),
                                open_handler_slot: open_handler_slot.clone(),
                                active_path: active_path.clone(),
                            },
                        );
                    }
                });
            }
        });
        app.add_action(&action);

        install_preferences_action(app, &sidebar);
        install_about_action(app, &window);

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn connect_navigation(
    sidebar: &Sidebar,
    stack: &gtk::Stack,
    title: &gtk::Label,
    back_button: &gtk::Button,
    active_preview: &Rc<RefCell<Option<gtk::Video>>>,
    detail: &gtk::Box,
    detail_controls: &Rc<RefCell<Option<DetailControls>>>,
) {
    for (button, page, page_title) in [
        (&sidebar.library_button, "library", "Biblioteca"),
        (&sidebar.displays_button, "displays", "Telas"),
        (&sidebar.settings_button, "settings", "Configurações"),
    ] {
        let stack = stack.clone();
        let title = title.clone();
        let back_button = back_button.clone();
        let active_preview = active_preview.clone();
        let detail = detail.clone();
        let detail_controls = detail_controls.clone();
        let sidebar = sidebar.clone();
        button.connect_clicked(move |_| {
            stop_preview(&active_preview, &detail);
            detail_controls.borrow_mut().take();
            stack.set_visible_child_name(page);
            title.set_label(page_title);
            back_button.set_visible(false);
            set_sidebar_selection(&sidebar, page);
        });
    }
}

fn set_sidebar_selection(sidebar: &Sidebar, page: &str) {
    for (button, selected) in [
        (&sidebar.library_button, page == "library"),
        (&sidebar.displays_button, page == "displays"),
        (&sidebar.settings_button, page == "settings"),
    ] {
        if selected {
            button.add_css_class("selected");
            button.add_css_class("nav-selected");
        } else {
            button.remove_css_class("selected");
            button.remove_css_class("nav-selected");
        }
    }
}

fn title_matches_query(title: &str, query: &str) -> bool {
    query.is_empty() || title.to_lowercase().contains(&query.to_lowercase())
}

fn displays_page(renderer: &RendererClient) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.set_margin_top(32);
    page.set_margin_bottom(32);
    page.set_margin_start(36);
    page.set_margin_end(36);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(1050);
    clamp.set_child(Some(&page));

    append_page_heading(
        &page,
        "CONFIGURAÇÃO DO DESKTOP",
        "Telas",
        "Informações da sessão e do renderer de wallpapers.",
    );

    let group = adw::PreferencesGroup::new();
    group.set_title("Integração atual");
    group.set_description(Some(
        "O GnomeEngine usa uma superfície de vídeo controlada pelo Mutter como fundo da sessão GNOME.",
    ));
    let session_row = adw::ActionRow::new();
    session_row.set_title("Sessão gráfica");
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "desconhecida".to_owned());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| "GNOME".to_owned());
    session_row.set_subtitle(&format!(
        "{} · {}",
        desktop.split(':').next().unwrap_or("GNOME"),
        session
    ));
    group.add(&session_row);
    let renderer_row = adw::ActionRow::new();
    renderer_row.set_title("Serviço do renderer");
    renderer_row.set_subtitle(if renderer.status().available {
        "Conectado à sessão D-Bus"
    } else {
        "Indisponível até a primeira aplicação de wallpaper"
    });
    let renderer_icon = gtk::Image::from_icon_name(if renderer.status().available {
        "emblem-ok-symbolic"
    } else {
        "dialog-warning-symbolic"
    });
    renderer_icon.set_valign(gtk::Align::Center);
    renderer_row.add_suffix(&renderer_icon);
    group.add(&renderer_row);
    let integration_row = adw::ActionRow::new();
    integration_row.set_title("Integração com o GNOME");
    integration_row.set_subtitle(crate::integration::guidance(
        renderer.status().desktop_integration_ready,
    ));
    let integration_icon =
        gtk::Image::from_icon_name(if renderer.status().desktop_integration_ready {
            "emblem-ok-symbolic"
        } else {
            "dialog-warning-symbolic"
        });
    integration_icon.set_valign(gtk::Align::Center);
    integration_row.add_suffix(&integration_icon);
    group.add(&integration_row);
    renderer.connect_status_changed({
        let renderer_row = renderer_row.clone();
        let renderer_icon = renderer_icon.clone();
        let integration_row = integration_row.clone();
        let integration_icon = integration_icon.clone();
        move |status| {
            renderer_row.set_subtitle(if status.available {
                "Conectado à sessão D-Bus"
            } else {
                "Indisponível até uma ação que precise do renderer"
            });
            renderer_icon.set_icon_name(Some(if status.available {
                "emblem-ok-symbolic"
            } else {
                "dialog-warning-symbolic"
            }));
            integration_row.set_subtitle(crate::integration::guidance(
                status.desktop_integration_ready,
            ));
            integration_icon.set_icon_name(Some(if status.desktop_integration_ready {
                "emblem-ok-symbolic"
            } else {
                "dialog-warning-symbolic"
            }));
        }
    });
    page.append(&group);

    let note = gtk::Label::new(Some(
        "O inventário de monitores ainda não é exposto ao aplicativo. Nenhum nome, resolução ou escala é estimado.",
    ));
    note.set_wrap(true);
    note.set_xalign(0.0);
    note.add_css_class("dim-label");
    page.append(&note);
    clamp.upcast()
}

fn settings_page(renderer: &RendererClient, toast_overlay: &adw::ToastOverlay) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.set_margin_top(32);
    page.set_margin_bottom(32);
    page.set_margin_start(36);
    page.set_margin_end(36);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(900);
    clamp.set_child(Some(&page));
    append_page_heading(
        &page,
        "PREFERÊNCIAS",
        "Configurações",
        "O GnomeEngine se adapta ao uso do seu computador.",
    );

    let group = adw::PreferencesGroup::new();
    group.set_title("Desempenho e energia");
    group.set_description(Some(
        "Comportamentos que ajudam a manter o desktop responsivo.",
    ));
    let battery = adw::SwitchRow::new();
    battery.set_title("Pausar usando bateria");
    battery.set_subtitle("Pausa o renderer enquanto o computador usa bateria.");
    battery.set_active(renderer.status().pause_on_battery);
    battery.set_sensitive(renderer.status().available);
    let reverting = Rc::new(std::cell::Cell::new(false));
    renderer.connect_status_changed({
        let battery = battery.clone();
        let reverting = reverting.clone();
        move |status| {
            battery.set_sensitive(status.available);
            if battery.is_active() != status.pause_on_battery {
                reverting.set(true);
                battery.set_active(status.pause_on_battery);
                reverting.set(false);
            }
        }
    });
    battery.connect_active_notify({
        let renderer = renderer.clone();
        let toast_overlay = toast_overlay.clone();
        let reverting = reverting.clone();
        move |row| {
            if reverting.get() {
                return;
            }
            let enabled = row.is_active();
            row.set_sensitive(false);
            renderer.set_pause_on_battery(enabled, {
                let row = row.clone();
                let toast_overlay = toast_overlay.clone();
                let reverting = reverting.clone();
                move |result| match result {
                    Ok(()) => {
                        row.set_sensitive(true);
                        toast_overlay.add_toast(adw::Toast::new("Preferência atualizada"));
                    }
                    Err(error) => {
                        eprintln!("gnomeengine: could not save lifecycle preference: {error}");
                        reverting.set(true);
                        row.set_active(!enabled);
                        reverting.set(false);
                        row.set_sensitive(true);
                        toast_overlay
                            .add_toast(adw::Toast::new("Não foi possível salvar esta preferência"));
                    }
                }
            });
        }
    });
    group.add(&battery);
    page.append(&group);

    let diagnostics = adw::PreferencesGroup::new();
    diagnostics.set_title("Diagnóstico");
    diagnostics.set_description(Some(
        "Estado atual da comunicação com o serviço de renderização.",
    ));
    let status = adw::ActionRow::new();
    status.set_title("Renderer");
    status.set_subtitle(&renderer_summary(&renderer.status()));
    let status_icon = gtk::Image::from_icon_name(if renderer.status().available {
        "emblem-ok-symbolic"
    } else {
        "dialog-warning-symbolic"
    });
    status_icon.set_valign(gtk::Align::Center);
    status.add_suffix(&status_icon);
    diagnostics.add(&status);
    renderer.connect_status_changed({
        let status = status.clone();
        let status_icon = status_icon.clone();
        move |snapshot| {
            status.set_subtitle(&renderer_summary(&snapshot));
            status_icon.set_icon_name(if snapshot.available {
                Some("emblem-ok-symbolic")
            } else {
                Some("dialog-warning-symbolic")
            });
        }
    });
    page.append(&diagnostics);
    clamp.upcast()
}

fn append_page_heading(parent: &gtk::Box, eyebrow: &str, title: &str, subtitle: &str) {
    let heading = gtk::Box::new(gtk::Orientation::Vertical, 7);
    let eyebrow_label = gtk::Label::new(Some(eyebrow));
    eyebrow_label.set_xalign(0.0);
    eyebrow_label.add_css_class("accent");
    eyebrow_label.add_css_class("caption");
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("title-1");
    let subtitle_label = gtk::Label::new(Some(subtitle));
    subtitle_label.set_xalign(0.0);
    subtitle_label.add_css_class("dim-label");
    subtitle_label.set_wrap(true);
    heading.append(&eyebrow_label);
    heading.append(&title_label);
    heading.append(&subtitle_label);
    parent.append(&heading);
}

fn renderer_summary(status: &RendererStatus) -> String {
    if !status.available {
        return "Serviço indisponível".to_owned();
    }
    match status.state.as_str() {
        "playing" => "Em reprodução".to_owned(),
        "paused" => status
            .pause_reasons
            .first()
            .map(|reason| format!("Pausado · {}", ui::localized_reason(reason)))
            .unwrap_or_else(|| "Pausado".to_owned()),
        "error" => "Erro na reprodução".to_owned(),
        "loading" | "loading-status" => "Carregando…".to_owned(),
        _ => "Sem wallpaper ativo".to_owned(),
    }
}

fn apply_error_message(error: &str) -> &'static str {
    if error.contains("GNOME desktop integration")
        || error.contains("GnomeEngine Shell extension")
        || error.contains("Renderer is incompatible")
    {
        if error.contains("Renderer is incompatible") {
            "Feche o renderer antigo e tente aplicar novamente"
        } else {
            crate::integration::guidance(false)
        }
    } else if error.contains("input-transparent") {
        "Esta sessão não permite uma superfície de wallpaper segura"
    } else {
        "Não foi possível aplicar o wallpaper"
    }
}

fn install_preferences_action(app: &adw::Application, sidebar: &Sidebar) {
    let action = gio::SimpleAction::new("preferences", None);
    let settings = sidebar.settings_button.clone();
    action.connect_activate(move |_, _| {
        settings.emit_clicked();
    });
    app.add_action(&action);
}

fn install_about_action(app: &adw::Application, parent: &adw::ApplicationWindow) {
    let action = gio::SimpleAction::new("about", None);
    action.connect_activate({
        let parent = parent.clone();
        move |_, _| {
            let about = adw::AboutWindow::new();
            about.set_application_name("GnomeEngine");
            about.set_version(env!("CARGO_PKG_VERSION"));
            about.set_developer_name("GnomeEngine contributors");
            about.set_website("https://github.com/mvk999/GnomeEngine");
            about.set_issue_url("https://github.com/mvk999/GnomeEngine/issues");
            about.set_transient_for(Some(&parent));
            about.present();
        }
    });
    app.add_action(&action);
}

fn update_renderer_status(
    label: &gtk::Label,
    stop_button: &gtk::Button,
    manual_pause_button: &gtk::Button,
    status: &RendererStatus,
) {
    if !status.available {
        label.set_label("Serviço do renderer indisponível");
        stop_button.set_visible(false);
        manual_pause_button.set_visible(false);
        return;
    }
    stop_button.set_visible(status.wallpaper_active);
    manual_pause_button.set_visible(status.wallpaper_active);
    let manual_paused = status.pause_reasons.iter().any(|reason| reason == "manual");
    manual_pause_button.set_icon_name(if manual_paused {
        "media-playback-start-symbolic"
    } else {
        "media-playback-pause-symbolic"
    });
    manual_pause_button.set_tooltip_text(Some(if manual_paused {
        "Retomar reprodução"
    } else {
        "Pausar reprodução"
    }));
    manual_pause_button.update_property(&[gtk::accessible::Property::Label(if manual_paused {
        "Retomar reprodução"
    } else {
        "Pausar reprodução"
    })]);
    let text = match status.state.as_str() {
        "playing" => "Em reprodução".to_owned(),
        "paused" if status.pause_reasons.is_empty() => "Pausado".to_owned(),
        "paused" => format!(
            "Pausado · {}",
            ui::localized_reason(&status.pause_reasons[0])
        ),
        "loading" | "loading-status" => "Carregando wallpaper…".to_owned(),
        "error" => "Erro na reprodução".to_owned(),
        "stopped" => "Renderer parado".to_owned(),
        _ => "Renderer conectado".to_owned(),
    };
    label.set_label(&text);
    label.set_tooltip_text(status.last_error.as_deref());
}

fn playback_control_result(
    button: gtk::Button,
    renderer: RendererClient,
) -> impl Fn(Result<(), String>) + 'static {
    move |result| {
        button.set_sensitive(renderer.status().available);
        if let Err(error) = result {
            eprintln!("gnomeengine: manual playback control failed: {error}");
        }
    }
}

fn stop_preview(active_preview: &Rc<RefCell<Option<gtk::Video>>>, detail: &gtk::Box) {
    if let Some(preview) = active_preview.borrow_mut().take() {
        preview.set_autoplay(false);
        if let Some(stream) = preview.media_stream() {
            stream.pause();
            stream.seek(0);
        }
        // The video is nested inside a frame/preview column, so releasing all
        // detail children is the correct way to detach its GTK media sink.
        while let Some(child) = detail.first_child() {
            detail.remove(&child);
        }
    }
}

fn library_view(
    library: &Library,
    on_open: Rc<dyn Fn(Wallpaper)>,
    active_path: Option<&str>,
) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 20);
    page.set_margin_top(34);
    page.set_margin_bottom(28);
    page.set_margin_start(38);
    page.set_margin_end(38);
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(1220);
    clamp.set_child(Some(&page));

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    heading.set_valign(gtk::Align::End);
    let heading_text = gtk::Box::new(gtk::Orientation::Vertical, 7);
    let eyebrow = gtk::Label::new(Some("SEU DESKTOP, DO SEU JEITO"));
    eyebrow.set_xalign(0.0);
    eyebrow.add_css_class("accent");
    eyebrow.add_css_class("caption");
    let title = gtk::Label::new(Some("Biblioteca"));
    title.set_xalign(0.0);
    title.add_css_class("title-1");
    let subtitle = gtk::Label::new(Some(if library.wallpapers().is_empty() {
        "Seus wallpapers ficam organizados em um só lugar."
    } else {
        "Escolha um wallpaper para visualizar ou aplicar."
    }));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("dim-label");
    subtitle.set_wrap(true);
    heading_text.append(&eyebrow);
    heading_text.append(&title);
    heading_text.append(&subtitle);
    heading.append(&heading_text);
    let import_button = gtk::Button::with_label("Importar wallpaper");
    import_button.set_icon_name("list-add-symbolic");
    import_button.add_css_class("suggested-action");
    import_button.set_action_name(Some("app.import"));
    import_button.set_tooltip_text(Some("Importar um vídeo para sua biblioteca"));
    import_button.update_property(&[gtk::accessible::Property::Label("Importar wallpaper")]);
    import_button.set_halign(gtk::Align::End);
    import_button.set_valign(gtk::Align::End);
    import_button.set_margin_bottom(2);
    heading.append(&import_button);
    page.append(&heading);

    if library.wallpapers().is_empty() {
        page.append(&empty_state());
    } else {
        page.append(&wallpaper_grid(library.wallpapers(), on_open, active_path));
    }
    clamp.upcast()
}

fn refresh_library_view(
    stack: &gtk::Stack,
    library: &Rc<RefCell<Library>>,
    open_handler_slot: &OpenHandlerSlot,
    active_path: Option<&str>,
) {
    if let Some(old_view) = stack.child_by_name("library") {
        stack.remove(&old_view);
    }
    if let Some(open_handler) = open_handler_slot.borrow().clone() {
        stack.add_named(
            &library_view(&library.borrow(), open_handler, active_path),
            Some("library"),
        );
    }
}

fn empty_state() -> gtk::Widget {
    let status = adw::StatusPage::builder()
        .icon_name("video-x-generic-symbolic")
        .title("Deixe seu desktop mais vivo")
        .description("Importe um vídeo para começar. Você poderá visualizar e aplicar seu wallpaper em poucos passos.")
        .build();
    let import_button = gtk::Button::with_label("Importar wallpaper");
    import_button.add_css_class("suggested-action");
    import_button.set_action_name(Some("app.import"));
    import_button.update_property(&[gtk::accessible::Property::Label("Importar wallpaper")]);
    status.set_child(Some(&import_button));
    status.upcast()
}

fn wallpaper_grid(
    wallpapers: &[Wallpaper],
    on_open: OpenHandler,
    active_path: Option<&str>,
) -> gtk::Widget {
    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    for wallpaper in wallpapers {
        store.append(&glib::BoxedAnyObject::new(wallpaper.clone()));
    }
    let query = Rc::new(RefCell::new(String::new()));
    let filter = gtk::CustomFilter::new({
        let query = query.clone();
        move |object| {
            let Ok(item) = object.clone().downcast::<glib::BoxedAnyObject>() else {
                return false;
            };
            let wallpaper = item.borrow::<Wallpaper>();
            title_matches_query(&wallpaper.manifest.title, &query.borrow())
        }
    });
    let filtered = gtk::FilterListModel::new(Some(store), Some(filter.clone()));
    let selection = gtk::NoSelection::new(Some(filtered.clone()));
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("card");
        card.add_css_class("wallpaper-card");
        card.set_margin_top(8);
        card.set_margin_bottom(8);
        card.set_margin_start(8);
        card.set_margin_end(8);
        card.set_accessible_role(gtk::AccessibleRole::Button);
        card.set_focusable(true);
        item.set_child(Some(&card));
    });
    factory.connect_bind({
        let active_path = active_path.map(str::to_owned);
        move |_, object| {
            let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
                return;
            };
            let (Some(item_object), Some(card)) = (
                item.item()
                    .and_then(|value| value.downcast::<glib::BoxedAnyObject>().ok()),
                item.child().and_downcast::<gtk::Box>(),
            ) else {
                return;
            };
            let wallpaper = item_object.borrow::<Wallpaper>();
            let active =
                active_path.as_deref() == Some(wallpaper.content_path.to_string_lossy().as_ref());
            let accessible_label = format!(
                "{}, vídeo, {} × {}, duração {}{}",
                wallpaper.manifest.title,
                wallpaper
                    .manifest
                    .media
                    .width
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "desconhecida".to_owned()),
                wallpaper
                    .manifest
                    .media
                    .height
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "desconhecida".to_owned()),
                wallpaper
                    .manifest
                    .media
                    .duration_seconds
                    .map(format_duration)
                    .unwrap_or_else(|| "desconhecida".to_owned()),
                if active { ", ativo no renderer" } else { "" },
            );
            card.update_property(&[gtk::accessible::Property::Label(&accessible_label)]);
            card.set_tooltip_text(Some(&wallpaper.manifest.title));
            while let Some(child) = card.first_child() {
                card.remove(&child);
            }
            populate_wallpaper_card(&card, &wallpaper, active_path.as_deref());
        }
    });
    factory.connect_unbind(|_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        if let Some(card) = item.child().and_downcast::<gtk::Box>() {
            while let Some(child) = card.first_child() {
                card.remove(&child);
            }
        }
    });
    let grid = gtk::GridView::new(Some(selection), Some(factory));
    grid.set_min_columns(1);
    grid.set_max_columns(4);
    grid.set_single_click_activate(true);
    grid.connect_activate(move |grid, position| {
        let Some(wallpaper) = grid
            .model()
            .and_then(|model| model.item(position))
            .and_then(|object| object.downcast::<glib::BoxedAnyObject>().ok())
            .map(|item| item.borrow::<Wallpaper>().clone())
        else {
            return;
        };
        on_open(wallpaper);
    });

    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let count = gtk::Label::new(Some(&format!("Todos ({})", wallpapers.len())));
    count.set_xalign(0.0);
    count.add_css_class("dim-label");
    let video_filter = gtk::Label::new(Some("Vídeos"));
    video_filter.add_css_class("caption");
    video_filter.add_css_class("dim-label");
    toolbar.append(&count);
    toolbar.append(&video_filter);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    toolbar.append(&spacer);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Buscar na biblioteca"));
    search.set_tooltip_text(Some("Buscar pelo título do wallpaper"));
    search.update_property(&[gtk::accessible::Property::Label("Buscar na biblioteca")]);
    search.connect_search_changed({
        let filter = filter.clone();
        move |search| {
            *query.borrow_mut() = search.text().trim().to_lowercase();
            filter.changed(gtk::FilterChange::Different);
        }
    });
    toolbar.append(&search);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 14);
    section.append(&toolbar);
    let scrolled = gtk::ScrolledWindow::builder()
        .child(&grid)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .build();
    section.append(&scrolled);
    section.upcast()
}

fn populate_wallpaper_card(card: &gtk::Box, wallpaper: &Wallpaper, active_path: Option<&str>) {
    let media_slot = gtk::Overlay::new();
    media_slot.set_size_request(-1, 170);
    if wallpaper.thumbnail_path.is_file() {
        let picture = gtk::Picture::for_filename(&wallpaper.thumbnail_path);
        picture.set_content_fit(gtk::ContentFit::Cover);
        picture.set_can_shrink(true);
        picture.set_hexpand(true);
        picture.set_vexpand(true);
        picture.set_accessible_role(gtk::AccessibleRole::Img);
        picture.update_property(&[gtk::accessible::Property::Label(
            wallpaper.manifest.title.as_str(),
        )]);
        media_slot.set_child(Some(&picture));
    } else {
        let placeholder = gtk::Image::from_icon_name("video-x-generic-symbolic");
        placeholder.set_pixel_size(48);
        placeholder.set_halign(gtk::Align::Center);
        placeholder.set_valign(gtk::Align::Center);
        placeholder.set_vexpand(true);
        placeholder.set_hexpand(true);
        placeholder.set_accessible_role(gtk::AccessibleRole::Img);
        placeholder.update_property(&[gtk::accessible::Property::Label(
            "Miniatura de vídeo indisponível",
        )]);
        media_slot.set_child(Some(&placeholder));
    }
    let badge = gtk::Label::new(Some("VÍDEO"));
    badge.add_css_class("caption");
    badge.add_css_class("osd");
    badge.set_halign(gtk::Align::Start);
    badge.set_valign(gtk::Align::Start);
    badge.set_margin_top(10);
    badge.set_margin_start(10);
    media_slot.add_overlay(&badge);
    media_slot.set_clip_overlay(&badge, true);
    let play = gtk::Image::from_icon_name("media-playback-start-symbolic");
    play.set_pixel_size(16);
    play.set_halign(gtk::Align::End);
    play.set_valign(gtk::Align::End);
    play.set_margin_end(11);
    play.set_margin_bottom(11);
    play.add_css_class("osd");
    play.update_property(&[gtk::accessible::Property::Label("Abrir pré-visualização")]);
    media_slot.add_overlay(&play);
    media_slot.set_clip_overlay(&play, true);
    card.append(&media_slot);

    let info = gtk::Box::new(gtk::Orientation::Vertical, 4);
    info.set_margin_top(12);
    info.set_margin_bottom(13);
    info.set_margin_start(14);
    info.set_margin_end(14);
    let title = gtk::Label::new(Some(&wallpaper.manifest.title));
    title.set_xalign(0.0);
    title.add_css_class("heading");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if active_path == Some(wallpaper.content_path.to_string_lossy().as_ref()) {
        title.set_label(&format!("{}  ·  Ativo", wallpaper.manifest.title));
        title.add_css_class("success");
        title.update_property(&[gtk::accessible::Property::Label(
            "Em reprodução no renderer",
        )]);
    }
    info.append(&title);
    let dimensions = match (
        wallpaper.manifest.media.width,
        wallpaper.manifest.media.height,
    ) {
        (Some(width), Some(height)) => format!("{width} × {height}"),
        _ => "Vídeo".to_owned(),
    };
    let duration = wallpaper
        .manifest
        .media
        .duration_seconds
        .map(format_duration)
        .unwrap_or_else(|| "—".to_owned());
    let subtitle = gtk::Label::new(Some(&format!("{dimensions}   ·   {duration}")));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("dim-label");
    subtitle.add_css_class("caption");
    info.append(&subtitle);
    card.append(&info);
}

fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "—".to_owned();
    }
    let total = seconds.round() as u64;
    format!("{:02}:{:02}", total / 60, total % 60)
}

fn format_file_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

fn show_detail(context: DetailContext, wallpaper: &Wallpaper) {
    let DetailContext {
        parent,
        stack,
        detail,
        header_title,
        back_button,
        active_preview,
        renderer,
        toast_overlay,
        library,
        open_handler_slot,
        active_path,
        detail_controls,
    } = context;
    while let Some(child) = detail.first_child() {
        detail.remove(&child);
    }
    let video = gtk::Video::for_filename(Some(&wallpaper.content_path));
    video.set_loop(true);
    if let Some(stream) = video.media_stream() {
        stream.set_muted(true);
        stream.connect_error_notify({
            let toast_overlay = toast_overlay.clone();
            move |stream| {
                if let Some(error) = stream.error() {
                    eprintln!("gnomeengine: preview failed: {error}");
                    toast_overlay.add_toast(adw::Toast::new("Pré-visualização indisponível"));
                }
            }
        });
    }
    video.set_autoplay(true);
    video.set_hexpand(true);
    video.set_size_request(420, 300);
    video.update_property(&[gtk::accessible::Property::Label(
        wallpaper.manifest.title.as_str(),
    )]);
    let preview_column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    preview_column.set_hexpand(true);
    preview_column.set_vexpand(true);
    let preview_frame = gtk::Frame::new(None);
    preview_frame.add_css_class("card");
    preview_frame.set_child(Some(&video));
    preview_column.append(&preview_frame);
    let preview_caption = gtk::Label::new(Some("PRÉ-VISUALIZAÇÃO · SEM ÁUDIO"));
    preview_caption.set_xalign(0.0);
    preview_caption.add_css_class("dim-label");
    preview_caption.add_css_class("caption");
    preview_column.append(&preview_caption);
    detail.append(&preview_column);

    let info = gtk::Box::new(gtk::Orientation::Vertical, 13);
    info.set_width_request(280);
    info.set_hexpand(false);
    let type_label = gtk::Label::new(Some("WALLPAPER · VÍDEO"));
    type_label.set_xalign(0.0);
    type_label.add_css_class("accent");
    type_label.add_css_class("caption");
    info.append(&type_label);
    let title_label = gtk::Label::new(Some(&wallpaper.manifest.title));
    title_label.add_css_class("title-1");
    title_label.set_xalign(0.0);
    title_label.set_wrap(true);
    info.append(&title_label);

    let specs = adw::PreferencesGroup::new();
    let resolution = match (
        wallpaper.manifest.media.width,
        wallpaper.manifest.media.height,
    ) {
        (Some(width), Some(height)) => format!("{width} × {height}"),
        _ => "—".to_owned(),
    };
    let fps = wallpaper
        .manifest
        .media
        .fps
        .map(|value| format!("{value:.0} fps"))
        .unwrap_or_else(|| "—".to_owned());
    let duration = wallpaper
        .manifest
        .media
        .duration_seconds
        .map(format_duration)
        .unwrap_or_else(|| "—".to_owned());
    let size = std::fs::metadata(&wallpaper.content_path)
        .ok()
        .map(|metadata| format_file_size(metadata.len()))
        .unwrap_or_else(|| "—".to_owned());
    for (label, value) in [
        ("Resolução", resolution),
        ("Taxa de quadros", fps),
        ("Duração", duration),
        ("Tamanho", size),
    ] {
        let row = adw::ActionRow::new();
        row.set_title(label);
        row.set_subtitle(&value);
        specs.add(&row);
    }
    info.append(&specs);

    let wallpaper_path = wallpaper.content_path.to_string_lossy().into_owned();
    let active = wallpaper_is_active(&renderer.status(), &wallpaper_path);
    let state_label = gtk::Label::new(Some(""));
    state_label.set_xalign(0.0);
    state_label.add_css_class("success");
    let apply = gtk::Button::with_label("Aplicar wallpaper");
    apply.add_css_class("suggested-action");
    apply.set_hexpand(true);
    apply.set_visible(!active);
    // Apply remains available while stopped; the client activates the service
    // and waits for the Shell extension to confirm desktop-surface readiness.
    apply.set_sensitive(apply_button_enabled(active));
    apply.set_tooltip_text(Some("Aplicar este wallpaper"));
    let stop = gtk::Button::with_label("Parar");
    stop.add_css_class("destructive-action");
    stop.set_hexpand(true);
    stop.set_visible(active);
    stop.set_sensitive(renderer.status().available);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    controls.append(&apply);
    controls.append(&stop);
    info.append(&state_label);
    info.append(&controls);

    let persistence_note = gtk::Label::new(Some(
        "A reprodução do renderer continua quando você fecha o GnomeEngine.",
    ));
    persistence_note.set_wrap(true);
    persistence_note.set_xalign(0.0);
    persistence_note.add_css_class("dim-label");
    persistence_note.add_css_class("caption");
    info.append(&persistence_note);

    let remove = gtk::Button::with_label("Remover da biblioteca…");
    remove.add_css_class("flat");
    remove.set_halign(gtk::Align::Start);
    info.append(&remove);
    detail.append(&info);

    *detail_controls.borrow_mut() = Some(DetailControls {
        path: wallpaper_path.clone(),
        apply: apply.clone(),
        stop: stop.clone(),
        status: state_label.clone(),
    });
    update_detail_controls(&detail_controls, &renderer.status());
    apply.connect_clicked({
        let renderer = renderer.clone();
        let toast_overlay = toast_overlay.clone();
        let apply = apply.clone();
        let path = wallpaper_path.clone();
        move |_| {
            apply.set_sensitive(false);
            renderer.apply_video(path.clone(), {
                let renderer = renderer.clone();
                let toast_overlay = toast_overlay.clone();
                let apply = apply.clone();
                move |result| {
                    apply.set_sensitive(true);
                    match result {
                        Ok(()) => {
                            renderer.refresh_status();
                            toast_overlay.add_toast(adw::Toast::new("Wallpaper aplicado"));
                        }
                        Err(error) => {
                            eprintln!("gnomeengine: apply wallpaper failed: {error}");
                            toast_overlay.add_toast(adw::Toast::new(apply_error_message(&error)));
                        }
                    }
                }
            });
        }
    });
    stop.connect_clicked({
        let renderer = renderer.clone();
        let toast_overlay = toast_overlay.clone();
        let stop = stop.clone();
        move |_| {
            stop.set_sensitive(false);
            renderer.stop({
                let renderer = renderer.clone();
                let toast_overlay = toast_overlay.clone();
                let stop = stop.clone();
                move |result| {
                    stop.set_sensitive(renderer.status().available);
                    match result {
                        Ok(()) => {
                            renderer.refresh_status();
                            toast_overlay.add_toast(adw::Toast::new("Wallpaper parado"));
                        }
                        Err(error) => {
                            eprintln!("gnomeengine: could not stop wallpaper: {error}");
                            toast_overlay
                                .add_toast(adw::Toast::new("Não foi possível parar o wallpaper"));
                        }
                    }
                }
            });
        }
    });

    remove.connect_clicked({
        let parent = parent.clone();
        let stack = stack.clone();
        let library = library.clone();
        let id = wallpaper.manifest.id.clone();
        let title = wallpaper.manifest.title.clone();
        let content_path = wallpaper.content_path.clone();
        let active_preview = active_preview.clone();
        let detail = detail.clone();
        let renderer = renderer.clone();
        let toast_overlay = toast_overlay.clone();
        let header_title = header_title.clone();
        let back_button = back_button.clone();
        let open_handler_slot = open_handler_slot.clone();
        let active_path = active_path.clone();
        let detail_controls = detail_controls.clone();
        move |_| {
            let dialog = adw::MessageDialog::new(
                Some(&parent),
                Some(&format!("Remover “{title}”?")),
                Some("A cópia importada será removida do GnomeEngine. O arquivo original não será alterado."),
            );
            dialog.add_response("cancel", "Cancelar");
            dialog.add_response("remove", "Remover");
            dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            dialog.connect_response(None, {
                let stack = stack.clone();
                let library = library.clone();
                let id = id.clone();
                let content_path = content_path.clone();
                let active_preview = active_preview.clone();
                let detail = detail.clone();
                let renderer = renderer.clone();
                let toast_overlay = toast_overlay.clone();
                let header_title = header_title.clone();
                let back_button = back_button.clone();
                let open_handler_slot = open_handler_slot.clone();
                let active_path = active_path.clone();
                let detail_controls = detail_controls.clone();
                move |_, response| {
                    if response != "remove" {
                        return;
                    }
                    stop_preview(&active_preview, &detail);
                    detail_controls.borrow_mut().take();
                    let finish = {
                        let id = id.clone();
                        let stack = stack.clone();
                        let library = library.clone();
                        let toast_overlay = toast_overlay.clone();
                        let header_title = header_title.clone();
                        let back_button = back_button.clone();
                        let open_handler_slot = open_handler_slot.clone();
                        let active_path = active_path.clone();
                        let content_path = content_path.to_string_lossy().into_owned();
                        move || {
                            remove_wallpaper(
                                id.clone(),
                                content_path.clone(),
                                RemovalContext {
                                    library: library.clone(),
                                    stack: stack.clone(),
                                    toast_overlay: toast_overlay.clone(),
                                    header_title: header_title.clone(),
                                    back_button: back_button.clone(),
                                    open_handler_slot: open_handler_slot.clone(),
                                    active_path: active_path.clone(),
                                },
                            );
                        }
                    };
                    let is_active = renderer.status().current_video.as_deref()
                        == Some(content_path.to_string_lossy().as_ref());
                    if is_active {
                        let toast_overlay = toast_overlay.clone();
                        renderer.stop(move |result| match result {
                            Ok(()) => finish(),
                            Err(error) => {
                                eprintln!("gnomeengine: could not stop active wallpaper before removal: {error}");
                                toast_overlay.add_toast(adw::Toast::new(
                                "Não foi possível parar o wallpaper ativo; o item não foi removido",
                                ));
                            }
                        });
                    } else {
                        finish();
                    }
                }
            });
            dialog.present();
        }
    });

    header_title.set_label("Detalhes do wallpaper");
    back_button.set_visible(true);
    stack.set_visible_child_name("detail");
    *active_preview.borrow_mut() = Some(video);
}

fn update_detail_controls(controls: &Rc<RefCell<Option<DetailControls>>>, status: &RendererStatus) {
    let Some(controls) = controls.borrow().as_ref().cloned() else {
        return;
    };
    let active = wallpaper_is_active(status, &controls.path);
    controls.apply.set_visible(!active);
    controls.stop.set_visible(active);
    controls.apply.set_sensitive(apply_button_enabled(active));
    controls.stop.set_sensitive(status.available && active);
    if active {
        controls.status.set_label(&renderer_summary(status));
        controls.status.set_visible(true);
    } else {
        controls.status.set_visible(false);
    }
}

fn wallpaper_is_active(status: &RendererStatus, path: &str) -> bool {
    status.wallpaper_active && status.current_video.as_deref() == Some(path)
}

fn apply_button_enabled(wallpaper_active: bool) -> bool {
    // An unavailable renderer can be activated by an explicit Apply request.
    !wallpaper_active
}

fn remove_wallpaper(id: String, deleted_path: String, context: RemovalContext) {
    let RemovalContext {
        library,
        stack,
        toast_overlay,
        header_title,
        back_button,
        open_handler_slot,
        active_path,
    } = context;
    let root = library.borrow().root().to_path_buf();
    toast_overlay.add_toast(adw::Toast::new("Removendo wallpaper…"));
    let (sender, receiver) = futures_channel::oneshot::channel();
    thread::spawn(move || {
        let result = Library::load(root).and_then(|mut library| {
            library.remove(&id)?;
            Ok(library)
        });
        let _ = sender.send(result);
    });
    glib::MainContext::default().spawn_local(async move {
        match receiver.await {
            Ok(Ok(updated)) => {
                *library.borrow_mut() = updated;
                if active_path.borrow().as_deref() == Some(deleted_path.as_str()) {
                    *active_path.borrow_mut() = None;
                }
                refresh_library_view(
                    &stack,
                    &library,
                    &open_handler_slot,
                    active_path.borrow().as_deref(),
                );
                stack.set_visible_child_name("library");
                header_title.set_label("Biblioteca");
                back_button.set_visible(false);
                toast_overlay.add_toast(adw::Toast::new("Wallpaper removido"));
            }
            Ok(Err(error)) => {
                eprintln!("gnomeengine: could not remove wallpaper: {error}");
                toast_overlay.add_toast(adw::Toast::new("Não foi possível remover o wallpaper"));
            }
            Err(_) => {
                eprintln!("gnomeengine: removal task ended unexpectedly");
                toast_overlay.add_toast(adw::Toast::new("A remoção não foi concluída"));
            }
        }
    });
}

fn start_import(source: PathBuf, library_root: PathBuf, context: ImportContext) {
    let ImportContext {
        parent,
        library,
        stack,
        toast_overlay,
        open_handler_slot,
        active_path,
    } = context;
    let progress = gtk::Window::builder()
        .title("Importando wallpaper")
        .transient_for(&parent)
        .modal(true)
        .deletable(false)
        .default_width(380)
        .build();
    let progress_content = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    progress_content.set_margin_top(28);
    progress_content.set_margin_bottom(28);
    progress_content.set_margin_start(28);
    progress_content.set_margin_end(28);
    let spinner = gtk::Spinner::new();
    spinner.start();
    progress_content.append(&spinner);
    let progress_label = gtk::Label::new(Some("Analisando e adicionando o vídeo à biblioteca…"));
    progress_label.set_wrap(true);
    progress_label.set_xalign(0.0);
    progress_content.append(&progress_label);
    progress.set_child(Some(&progress_content));
    progress.present();
    let (sender, receiver) = futures_channel::oneshot::channel();
    thread::spawn(move || {
        let result = import_video(&source, &library_root);
        let _ = sender.send(result);
    });
    glib::MainContext::default().spawn_local(async move {
        let result = receiver.await;
        progress.close();
        match result {
            Ok(Ok(wallpaper)) => {
                let result = library.borrow_mut().include_imported(wallpaper);
                let result = result.or_else(|insert_error| {
                    eprintln!(
                        "gnomeengine: imported item could not be inserted directly; reloading library: {insert_error}"
                    );
                    let root = library.borrow().root().to_path_buf();
                    Library::load(root).map(|updated| *library.borrow_mut() = updated)
                });
                match result {
                    Ok(()) => {
                        refresh_library_view(
                            &stack,
                            &library,
                            &open_handler_slot,
                            active_path.borrow().as_deref(),
                        );
                        stack.set_visible_child_name("library");
                        toast_overlay.add_toast(adw::Toast::new("Wallpaper importado"));
                    }
                    Err(error) => {
                        eprintln!("gnomeengine: could not add imported wallpaper to live library: {error}");
                        toast_overlay.add_toast(adw::Toast::new(
                            "Wallpaper importado, mas a biblioteca não pôde ser atualizada",
                        ));
                    }
                }
            }
            Ok(Err(error)) => {
                eprintln!("gnomeengine: import failed: {error}");
                toast_overlay.add_toast(adw::Toast::new(
                    "Não foi possível importar o vídeo. Verifique se o arquivo contém vídeo legível.",
                ));
            }
            Err(_) => {
                eprintln!("gnomeengine: import task ended unexpectedly");
                toast_overlay.add_toast(adw::Toast::new("A importação não foi concluída"));
            }
        }
    });
}

#[cfg(test)]
mod presentation_tests {
    use super::{
        apply_button_enabled, apply_error_message, format_duration, format_file_size,
        title_matches_query, wallpaper_is_active,
    };
    use crate::renderer_client::RendererStatus;

    #[test]
    fn apply_remains_available_while_renderer_is_stopped() {
        let status = RendererStatus::default();
        assert!(!status.available);
        assert!(!wallpaper_is_active(&status, "/library/wallpaper"));
        assert!(apply_button_enabled(false));
    }

    #[test]
    fn apply_is_replaced_by_stop_for_the_active_wallpaper() {
        assert!(!apply_button_enabled(true));
    }

    #[test]
    fn apply_error_uses_host_specific_gnome_integration_guidance() {
        assert_eq!(
            apply_error_message("GNOME desktop integration is not ready"),
            crate::integration::guidance(false)
        );
    }

    #[test]
    fn apply_error_explains_when_an_old_renderer_is_still_running() {
        assert_eq!(
            apply_error_message("Renderer is incompatible with desktop integration"),
            "Feche o renderer antigo e tente aplicar novamente"
        );
    }

    #[test]
    fn formats_only_available_media_values_without_inventing_metadata() {
        assert_eq!(format_duration(24.3), "00:24");
        assert_eq!(format_duration(f64::NAN), "—");
        assert_eq!(format_file_size(148 * 1024 * 1024), "148.0 MB");
        assert_eq!(format_file_size(512), "512 B");
        assert!(title_matches_query("Aurora Boreal", "aurora"));
        assert!(!title_matches_query("Aurora Boreal", "ocean"));
        assert!(title_matches_query("Aurora Boreal", ""));
    }
}
