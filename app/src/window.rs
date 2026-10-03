use std::{cell::RefCell, path::PathBuf, rc::Rc, thread};

use adw::prelude::*;

use crate::library::{import_video, Library, Wallpaper};
use crate::renderer_client::{RendererClient, RendererStatus};

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

pub struct MainWindow {
    window: adw::ApplicationWindow,
}

impl MainWindow {
    pub fn new(app: &adw::Application, library: Library, renderer: RendererClient) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("GnomeEngine")
            .default_width(1080)
            .default_height(720)
            .build();

        let toolbar = adw::ToolbarView::new();
        let toast_overlay = adw::ToastOverlay::new();
        let header = adw::HeaderBar::new();
        let title_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let header_title = gtk::Label::new(Some("Library"));
        header_title.add_css_class("title");
        let renderer_status_label = gtk::Label::new(Some("Connecting to renderer…"));
        renderer_status_label.add_css_class("dim-label");
        renderer_status_label.add_css_class("caption");
        title_box.append(&header_title);
        title_box.append(&renderer_status_label);
        header.set_title_widget(Some(&title_box));

        let stop_button = gtk::Button::from_icon_name("media-playback-stop-symbolic");
        stop_button.set_tooltip_text(Some("Stop Renderer Playback"));
        stop_button.update_property(&[gtk::accessible::Property::Label("Stop renderer playback")]);
        stop_button.set_visible(false);
        header.pack_end(&stop_button);
        let manual_pause_button = gtk::Button::from_icon_name("media-playback-pause-symbolic");
        manual_pause_button.set_tooltip_text(Some("Pause Renderer Playback"));
        manual_pause_button
            .update_property(&[gtk::accessible::Property::Label("Pause renderer playback")]);
        manual_pause_button.set_visible(false);
        header.pack_end(&manual_pause_button);

        let menu = gio::Menu::new();
        menu.append(Some("Preferences"), Some("app.preferences"));
        menu.append(Some("About GnomeEngine"), Some("app.about"));
        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Main Menu")
            .menu_model(&menu)
            .build();
        menu_button.update_property(&[gtk::accessible::Property::Label("Main menu")]);
        header.pack_end(&menu_button);
        let import_button = gtk::Button::from_icon_name("list-add-symbolic");
        import_button.set_tooltip_text(Some("Import Wallpaper"));
        import_button.update_property(&[gtk::accessible::Property::Label("Import wallpaper")]);
        import_button.set_action_name(Some("app.import"));
        header.pack_end(&import_button);
        toolbar.add_top_bar(&header);

        let library = Rc::new(RefCell::new(library));
        if library.borrow().invalid_items() > 0 {
            toast_overlay.add_toast(adw::Toast::new("Some library items could not be loaded"));
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
                        Ok(()) => {
                            toast_overlay.add_toast(adw::Toast::new("Renderer playback stopped"))
                        }
                        Err(error) => {
                            eprintln!("gnomeengine: could not stop wallpaper: {error}");
                            toast_overlay.add_toast(adw::Toast::new("Could not stop wallpaper"));
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

        let detail_view = gtk::Box::new(gtk::Orientation::Vertical, 12);
        detail_view.set_margin_top(24);
        detail_view.set_margin_bottom(24);
        detail_view.set_margin_start(24);
        detail_view.set_margin_end(24);
        stack.add_named(&detail_view, Some("detail"));
        let back_button = gtk::Button::from_icon_name("go-previous-symbolic");
        back_button.set_tooltip_text(Some("Back to Library"));
        back_button.update_property(&[gtk::accessible::Property::Label("Back to Library")]);
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
        toolbar.set_content(Some(&stack));
        toast_overlay.set_child(Some(&toolbar));
        window.set_content(Some(&toast_overlay));

        {
            let renderer_status_label = renderer_status_label.clone();
            let stop_button = stop_button.clone();
            let manual_pause_button = manual_pause_button.clone();
            let stack = stack.clone();
            let library = library.clone();
            let open_handler_slot = open_handler_slot.clone();
            let active_path = active_path.clone();
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
            back_button.connect_clicked(move |_| {
                stop_preview(&active_preview, &detail_view);
                stack.set_visible_child_name("library");
                refresh_library_view(
                    &stack,
                    &library,
                    &open_handler_slot,
                    active_path.borrow().as_deref(),
                );
                header_title.set_label("Library");
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
            move |_, _| {
                stop_preview(&active_preview, &detail_view);
                stack.set_visible_child_name("library");
                header_title.set_label("Library");
                back_button.set_visible(false);

                let dialog = gtk::FileDialog::builder().title("Import Wallpaper").build();
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("Video files"));
                filter.add_mime_type("video/*");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));
                dialog.set_default_filter(Some(&filter));

                dialog.open(Some(&window), gio::Cancellable::NONE, {
                    let library = library.clone();
                    let stack = stack.clone();
                    let toast_overlay = toast_overlay.clone();
                    let open_handler = open_handler.clone();
                    let active_path = active_path.clone();
                    move |result| {
                        let file = match result {
                            Ok(file) => file,
                            Err(error) if error.matches(gio::IOErrorEnum::Cancelled) => return,
                            Err(error) => {
                                eprintln!("gnomeengine: file chooser failed: {error}");
                                toast_overlay.add_toast(adw::Toast::new(&format!(
                                    "Could not open file chooser: {error}"
                                )));
                                return;
                            }
                        };
                        let Some(source) = file.path() else {
                            toast_overlay.add_toast(adw::Toast::new("Choose a local video file"));
                            return;
                        };
                        start_import(
                            source,
                            library.borrow().root().to_path_buf(),
                            library.clone(),
                            stack.clone(),
                            toast_overlay.clone(),
                            open_handler.clone(),
                            active_path.borrow().as_deref().map(str::to_owned),
                        );
                    }
                });
            }
        });
        app.add_action(&action);

        install_preferences_action(app, &window, &renderer, &toast_overlay);
        install_about_action(app, &window);

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn install_preferences_action(
    app: &adw::Application,
    parent: &adw::ApplicationWindow,
    renderer: &RendererClient,
    toast_overlay: &adw::ToastOverlay,
) {
    let action = gio::SimpleAction::new("preferences", None);
    action.connect_activate({
        let parent = parent.clone();
        let renderer = renderer.clone();
        let toast_overlay = toast_overlay.clone();
        move |_, _| {
            let window = adw::PreferencesWindow::new();
            window.set_title(Some("Preferences"));
            window.set_transient_for(Some(&parent));
            let page = adw::PreferencesPage::new();
            let group = adw::PreferencesGroup::new();
            group.set_title("Performance");
            let row = adw::SwitchRow::new();
            row.set_title("Pause on battery");
            row.set_subtitle("Stop video rendering while the computer uses battery power.");
            row.set_active(renderer.status().pause_on_battery);
            row.set_sensitive(renderer.status().available);
            let reverting = Rc::new(std::cell::Cell::new(false));
            renderer.connect_status_changed({
                let row = row.clone();
                let reverting = reverting.clone();
                move |status| {
                    row.set_sensitive(status.available);
                    if row.is_active() != status.pause_on_battery {
                        reverting.set(true);
                        row.set_active(status.pause_on_battery);
                        reverting.set(false);
                    }
                }
            });
            row.connect_active_notify({
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
                            Ok(()) => row.set_sensitive(true),
                            Err(error) => {
                                eprintln!(
                                    "gnomeengine: could not save lifecycle preference: {error}"
                                );
                                reverting.set(true);
                                row.set_active(!enabled);
                                reverting.set(false);
                                row.set_sensitive(true);
                                toast_overlay
                                    .add_toast(adw::Toast::new("Could not save this preference"));
                            }
                        }
                    });
                }
            });
            group.add(&row);
            page.add(&group);
            window.add(&page);
            window.present();
        }
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
        label.set_label("Renderer service unavailable");
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
        "Resume Renderer Playback"
    } else {
        "Pause Renderer Playback"
    }));
    manual_pause_button.update_property(&[gtk::accessible::Property::Label(if manual_paused {
        "Resume renderer playback"
    } else {
        "Pause renderer playback"
    })]);
    let text = match status.state.as_str() {
        "playing" => "Renderer · Playing".to_owned(),
        "paused" if status.pause_reasons.is_empty() => "Renderer · Paused".to_owned(),
        "paused" => format!("Renderer · Paused · {}", status.pause_reasons.join(", ")),
        "loading" | "loading-status" => "Loading wallpaper…".to_owned(),
        "error" => "Playback error".to_owned(),
        "stopped" => "Renderer · Stopped".to_owned(),
        _ => "Renderer connected".to_owned(),
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
        if preview.parent().is_some() {
            detail.remove(&preview);
        }
    }
}

fn library_view(
    library: &Library,
    on_open: Rc<dyn Fn(Wallpaper)>,
    active_path: Option<&str>,
) -> gtk::Widget {
    if library.wallpapers().is_empty() {
        empty_state()
    } else {
        wallpaper_grid(library.wallpapers(), on_open, active_path)
    }
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
        .title("No wallpapers yet")
        .description("Import a video to start building your local wallpaper library.")
        .build();
    let import_button = gtk::Button::with_label("Import Wallpaper");
    import_button.add_css_class("suggested-action");
    import_button.set_action_name(Some("app.import"));
    import_button.update_property(&[gtk::accessible::Property::Label("Import wallpaper")]);
    status.set_child(Some(&import_button));
    status.upcast()
}

fn wallpaper_grid(
    wallpapers: &[Wallpaper],
    on_open: OpenHandler,
    active_path: Option<&str>,
) -> gtk::Widget {
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .max_children_per_line(5)
        .min_children_per_line(2)
        .row_spacing(18)
        .column_spacing(18)
        .homogeneous(true)
        .build();
    flow.set_margin_top(24);
    flow.set_margin_bottom(24);
    flow.set_margin_start(24);
    flow.set_margin_end(24);

    for wallpaper in wallpapers {
        let card = gtk::Button::new();
        card.set_has_frame(false);
        card.add_css_class("card");
        card.set_hexpand(true);
        card.set_valign(gtk::Align::Start);
        card.set_accessible_role(gtk::AccessibleRole::Button);
        card.update_property(&[gtk::accessible::Property::Label(
            wallpaper.manifest.title.as_str(),
        )]);
        let box_ = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let media_slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
        media_slot.set_size_request(-1, 150);
        if wallpaper.thumbnail_path.is_file() {
            let picture = gtk::Picture::for_filename(&wallpaper.thumbnail_path);
            picture.set_height_request(150);
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture.set_can_shrink(true);
            picture.set_accessible_role(gtk::AccessibleRole::Img);
            picture.update_property(&[gtk::accessible::Property::Label(
                wallpaper.manifest.title.as_str(),
            )]);
            media_slot.append(&picture);
        } else {
            let placeholder = gtk::Image::from_icon_name("video-x-generic-symbolic");
            placeholder.set_pixel_size(48);
            placeholder.set_halign(gtk::Align::Center);
            placeholder.set_valign(gtk::Align::Center);
            placeholder.set_vexpand(true);
            placeholder.set_accessible_role(gtk::AccessibleRole::Img);
            placeholder.update_property(&[gtk::accessible::Property::Label(
                "Video thumbnail unavailable",
            )]);
            media_slot.append(&placeholder);
        }
        box_.append(&media_slot);
        let title = gtk::Label::new(Some(&wallpaper.manifest.title));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        box_.append(&title);
        if active_path == Some(wallpaper.content_path.to_string_lossy().as_ref()) {
            let active = gtk::Label::new(Some("Active"));
            active.set_xalign(0.0);
            active.add_css_class("success");
            active.update_property(&[gtk::accessible::Property::Label("Active in renderer")]);
            active.set_label("Active in renderer");
            box_.append(&active);
        }
        let dimensions = match (
            wallpaper.manifest.media.width,
            wallpaper.manifest.media.height,
        ) {
            (Some(width), Some(height)) => format!("Video · {width} × {height}"),
            _ => "Video".to_owned(),
        };
        let subtitle = gtk::Label::new(Some(&dimensions));
        subtitle.set_xalign(0.0);
        subtitle.add_css_class("dim-label");
        box_.append(&subtitle);
        card.set_child(Some(&box_));
        let wallpaper = wallpaper.clone();
        let on_open = on_open.clone();
        card.connect_clicked(move |_| {
            on_open(wallpaper.clone());
        });
        flow.insert(&card, -1);
    }

    let scrolled = gtk::ScrolledWindow::builder().child(&flow).build();
    scrolled.upcast()
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
                    toast_overlay.add_toast(adw::Toast::new("Preview unavailable"));
                }
            }
        });
    }
    video.set_autoplay(true);
    video.set_hexpand(true);
    video.set_vexpand(true);
    video.set_size_request(-1, 270);
    video.update_property(&[gtk::accessible::Property::Label(
        wallpaper.manifest.title.as_str(),
    )]);
    detail.append(&video);

    let title_label = gtk::Label::new(Some(&wallpaper.manifest.title));
    title_label.add_css_class("title-1");
    title_label.set_xalign(0.0);
    detail.append(&title_label);

    let integration_note = gtk::Label::new(Some(
        "Desktop background integration is not implemented yet. Apply opens the renderer preview window.",
    ));
    integration_note.set_xalign(0.0);
    integration_note.set_wrap(true);
    integration_note.add_css_class("dim-label");
    detail.append(&integration_note);

    let apply = gtk::Button::with_label("Apply Wallpaper");
    apply.add_css_class("suggested-action");
    apply.set_halign(gtk::Align::End);
    apply.set_sensitive(true);
    apply.set_tooltip_text(Some("Apply this wallpaper to the renderer"));
    apply.connect_clicked({
        let renderer = renderer.clone();
        let toast_overlay = toast_overlay.clone();
        let apply = apply.clone();
        let path = wallpaper.content_path.to_string_lossy().into_owned();
        move |_| {
            apply.set_sensitive(false);
            renderer.apply_video(path.clone(), {
                let renderer = renderer.clone();
                let toast_overlay = toast_overlay.clone();
                let apply = apply.clone();
                move |result| {
                    apply.set_sensitive(renderer.status().available);
                    match result {
                        Ok(()) => toast_overlay
                            .add_toast(adw::Toast::new("Renderer started video playback")),
                        Err(error) => {
                            eprintln!("gnomeengine: apply wallpaper failed: {error}");
                            toast_overlay.add_toast(adw::Toast::new(&format!(
                                "Could not apply wallpaper: {error}"
                            )));
                        }
                    }
                }
            });
        }
    });
    detail.append(&apply);

    let remove = gtk::Button::with_label("Remove from Library…");
    remove.set_halign(gtk::Align::End);
    remove.add_css_class("destructive-action");
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
        move |_| {
            let dialog = adw::MessageDialog::new(
                Some(&parent),
                Some(&format!("Remove “{title}”?")),
                Some("The imported copy will be deleted from GnomeEngine. The original file will not be affected."),
            );
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("remove", "Remove");
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
                move |_, response| {
                    if response != "remove" {
                        return;
                    }
                    stop_preview(&active_preview, &detail);
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
                                    "Could not stop active wallpaper; item was not removed",
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
    detail.append(&remove);

    header_title.set_label(&wallpaper.manifest.title);
    back_button.set_visible(true);
    stack.set_visible_child_name("detail");
    *active_preview.borrow_mut() = Some(video);
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
    toast_overlay.add_toast(adw::Toast::new("Removing wallpaper…"));
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
                header_title.set_label("Library");
                back_button.set_visible(false);
                toast_overlay.add_toast(adw::Toast::new("Wallpaper removed"));
            }
            Ok(Err(error)) => {
                eprintln!("gnomeengine: could not remove wallpaper: {error}");
                toast_overlay.add_toast(adw::Toast::new("Could not remove wallpaper"));
            }
            Err(_) => {
                eprintln!("gnomeengine: removal task ended unexpectedly");
                toast_overlay.add_toast(adw::Toast::new("Wallpaper removal did not finish"));
            }
        }
    });
}

fn start_import(
    source: PathBuf,
    library_root: PathBuf,
    library: Rc<RefCell<Library>>,
    stack: gtk::Stack,
    toast_overlay: adw::ToastOverlay,
    on_open: OpenHandler,
    active_path: Option<String>,
) {
    toast_overlay.add_toast(adw::Toast::new("Importing wallpaper…"));
    let (sender, receiver) = futures_channel::oneshot::channel();
    thread::spawn(move || {
        let result = import_video(&source, &library_root);
        let _ = sender.send(result);
    });
    glib::MainContext::default().spawn_local(async move {
        match receiver.await {
            Ok(Ok(_wallpaper)) => match Library::load(library.borrow().root().to_path_buf()) {
                Ok(updated) => {
                    *library.borrow_mut() = updated;
                    if let Some(old_library_view) = stack.child_by_name("library") {
                        stack.remove(&old_library_view);
                    }
                    stack.add_named(
                        &library_view(&library.borrow(), on_open, active_path.as_deref()),
                        Some("library"),
                    );
                    stack.set_visible_child_name("library");
                    toast_overlay.add_toast(adw::Toast::new("Wallpaper imported"));
                }
                Err(error) => {
                    eprintln!("gnomeengine: could not refresh imported library: {error}");
                    toast_overlay.add_toast(adw::Toast::new(
                        "Wallpaper imported, but library could not refresh",
                    ));
                }
            },
            Ok(Err(error)) => {
                eprintln!("gnomeengine: import failed: {error}");
                toast_overlay
                    .add_toast(adw::Toast::new(&format!("Could not import video: {error}")));
            }
            Err(_) => {
                eprintln!("gnomeengine: import task ended unexpectedly");
                toast_overlay.add_toast(adw::Toast::new("Import did not finish"));
            }
        }
    });
}
