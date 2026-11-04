use crate::gtk::{self as g, c, Widget};
use gio::prelude::*;
use glib::{translate::*, ToValue};
use luma_launcher::{
    search::{self, Entry},
    state::{self, Config, State, Usage},
    system, tools,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    ffi::CStr,
    rc::Rc,
};

fn object(p: Widget) -> glib::Object {
    unsafe { from_glib_none(p.cast::<glib::gobject_ffi::GObject>()) }
}
fn signal(p: Widget, name: &str, f: impl Fn(&[glib::Value]) -> Option<glib::Value> + 'static) {
    object(p).connect_local(name, false, f);
}
unsafe fn label(s: &str) -> Widget {
    let w = g::gtk_label_new(c(s).as_ptr());
    g::gtk_label_set_xalign(w, 0.);
    g::gtk_label_set_ellipsize(w, 3);
    w
}
fn discover() -> Vec<Entry> {
    let mut seen = std::collections::BTreeSet::new();
    gio::AppInfo::all()
        .into_iter()
        .filter(|a| a.should_show())
        .filter_map(|a| {
            let id = a.id()?.to_string();
            if !seen.insert(id.clone()) {
                return None;
            }
            let d = gio::DesktopAppInfo::new(&id);
            let fields = d
                .as_ref()
                .map(|d| {
                    format!(
                        "{} {} {}",
                        d.generic_name().unwrap_or_default(),
                        d.keywords().join(" "),
                        d.categories().unwrap_or_default()
                    )
                })
                .unwrap_or_default();
            Some(Entry::new(
                id,
                a.display_name().into(),
                a.description().unwrap_or_default().into(),
                a.icon()
                    .and_then(|i| gio::prelude::IconExt::to_string(&i))
                    .unwrap_or_default()
                    .into(),
                fields,
            ))
        })
        .collect()
}
#[derive(Clone)]
pub struct Launcher(Rc<Inner>);
struct Inner {
    app: gio::Application,
    _hold: gio::ApplicationHoldGuard,
    window: Widget,
    input: Widget,
    settings_button: Widget,
    list: Widget,
    scroll: Widget,
    status: Widget,
    settings_status: Cell<Widget>,
    entries: RefCell<Vec<Entry>>,
    results: RefCell<Vec<usize>>,
    selected: Cell<i32>,
    visible: Cell<bool>,
    config_writable: bool,
    state_writable: bool,
    config: RefCell<Config>,
    state: RefCell<State>,
    system: RefCell<BTreeMap<String, Usage>>,
    system_monitor: RefCell<Option<gio::FileMonitor>>,
    provider: Widget,
    settings: Cell<Widget>,
    indexing: Cell<bool>,
    refresh_pending: Cell<bool>,
    monitor: gio::AppInfoMonitor,
    theme_settings: Option<gio::Settings>,
    portal_pending: Cell<bool>,
    portal: RefCell<Option<crate::portal::Session>>,
}
impl Launcher {
    pub fn new(app: &gio::Application) -> Self {
        unsafe {
            let config = state::load::<Config>(&state::directory("config").join("config.toml"))
                .and_then(|c| {
                    c.validate()?;
                    Ok(c)
                });
            let history = state::load::<State>(&state::directory("state").join("history.toml"));
            let error = config.as_ref().err().or(history.as_ref().err()).cloned();
            let window = g::gtk_application_window_new(app.as_ptr().cast());
            g::gtk_widget_add_css_class(window, c("luma").as_ptr());
            g::gtk_widget_add_css_class(window, c("launcher").as_ptr());
            g::gtk_window_set_title(window, c("Luma Launcher").as_ptr());
            g::gtk_window_set_decorated(window, 0);
            let layout = g::gtk_box_new(1, 0);
            g::gtk_widget_add_css_class(layout, c("palette").as_ptr());
            g::gtk_window_set_child(window, layout);
            let input = g::gtk_entry_new();
            g::gtk_widget_add_css_class(input, c("search").as_ptr());
            g::gtk_entry_set_max_length(input, 512);
            g::gtk_entry_set_placeholder_text(input, c("Search applications...").as_ptr());
            g::gtk_entry_set_icon_from_icon_name(input, 0, c("system-search-symbolic").as_ptr());
            let search_bar = g::gtk_box_new(0, 8);
            g::gtk_widget_add_css_class(search_bar, c("search-bar").as_ptr());
            g::gtk_widget_set_hexpand(input, 1);
            g::gtk_box_append(search_bar, input);
            let settings_button = button("Settings");
            g::gtk_widget_set_tooltip_text(
                settings_button,
                c("Open Settings (Ctrl+, or Ctrl++)").as_ptr(),
            );
            g::gtk_box_append(search_bar, settings_button);
            g::gtk_box_append(layout, search_bar);
            let list = g::gtk_list_box_new();
            let scroll = g::gtk_scrolled_window_new();
            g::gtk_scrolled_window_set_child(scroll, list);
            g::gtk_scrolled_window_set_max_content_height(scroll, 380);
            g::gtk_scrolled_window_set_propagate_natural_height(scroll, 1);
            g::gtk_box_append(layout, scroll);
            let status = label("↑↓ Navigate    Enter Open    Esc Close");
            g::gtk_label_set_ellipsize(status, 0);
            g::gtk_label_set_wrap(status, 1);
            g::gtk_widget_add_css_class(status, c("footer").as_ptr());
            g::gtk_box_append(layout, status);
            let provider = g::gtk_css_provider_new();
            g::gtk_style_context_add_provider_for_display(
                g::gtk_widget_get_display(window),
                provider,
                600,
            );
            let this = Self(Rc::new(Inner {
                app: app.clone(),
                _hold: app.hold(),
                window,
                input,
                settings_button,
                list,
                scroll,
                status,
                settings_status: Cell::new(std::ptr::null_mut()),
                entries: RefCell::new(vec![]),
                results: RefCell::new(vec![]),
                selected: Cell::new(0),
                visible: Cell::new(false),
                config_writable: config.is_ok(),
                state_writable: history.is_ok(),
                config: RefCell::new(config.unwrap_or_default()),
                state: RefCell::new(history.unwrap_or_default()),
                system: RefCell::new(BTreeMap::new()),
                system_monitor: RefCell::new(None),
                provider,
                settings: Cell::new(std::ptr::null_mut()),
                indexing: Cell::new(false),
                refresh_pending: Cell::new(false),
                monitor: gio::AppInfoMonitor::get(),
                theme_settings: system_settings(),
                portal_pending: Cell::new(false),
                portal: RefCell::new(None),
            }));
            if let Some(settings) = &this.0.theme_settings {
                let t = this.clone();
                settings.connect_changed(Some("color-scheme"), move |_, _| t.theme());
            }
            this.theme();
            let t = this.clone();
            signal(settings_button, "clicked", move |_| {
                t.settings();
                None
            });
            let t = this.clone();
            signal(input, "changed", move |_| {
                t.render();
                None
            });
            let t = this.clone();
            signal(input, "activate", move |_| {
                t.launch();
                None
            });
            let t = this.clone();
            signal(list, "row-activated", move |v| {
                let row = v[1].get::<glib::Object>().unwrap();
                t.0.selected
                    .set(g::gtk_list_box_row_get_index(row.as_ptr().cast()));
                t.launch();
                None
            });
            let t = this.clone();
            signal(list, "row-selected", move |v| {
                if let Ok(Some(row)) = v[1].get::<Option<glib::Object>>() {
                    t.0.selected
                        .set(g::gtk_list_box_row_get_index(row.as_ptr().cast()));
                }
                None
            });
            let t = this.clone();
            signal(window, "close-request", move |_| {
                t.hide();
                Some(true.to_value())
            });
            let t = this.clone();
            object(window).connect_notify_local(Some("is-active"), move |window, _| {
                if !window.property::<bool>("is-active") {
                    t.hide();
                }
            });
            let keys = key_controller();
            g::gtk_widget_add_controller(window, keys);
            let t = this.clone();
            signal(keys, "key-pressed", move |v| {
                let key = v[1].get::<u32>().unwrap_or(0);
                let mods = glib::gobject_ffi::g_value_get_flags(v[3].to_glib_none().0);
                let ctrl = mods & 4 != 0;
                let handled = match key {
                    65293 if ctrl => {
                        t.actions();
                        true
                    }
                    65307 => {
                        t.hide();
                        true
                    }
                    65364 => {
                        t.navigate(1);
                        true
                    }
                    65362 => {
                        t.navigate(-1);
                        true
                    }
                    106 if ctrl => {
                        t.navigate(1);
                        true
                    }
                    107 if ctrl => {
                        t.navigate(-1);
                        true
                    }
                    _ if settings_shortcut(key, ctrl) => {
                        t.settings();
                        true
                    }
                    100 if ctrl => {
                        t.favorite();
                        true
                    }
                    65535 if ctrl => {
                        t.forget();
                        true
                    }
                    _ => false,
                };
                Some(handled.to_value())
            });
            let monitor = &this.0.monitor;
            let t = this.clone();
            monitor.connect_changed(move |_| t.refresh());
            if this.0.config.borrow().system_usage {
                this.harvest();
                if let Some((_, path)) = system::backend() {
                    // GNOME replaces the file atomically and KDE activity
                    // statistics write through -wal, so watch the directory.
                    let watch = match path.parent() {
                        Some(parent) => gio::File::for_path(parent),
                        None => gio::File::for_path(&path),
                    };
                    if let Ok(monitor) =
                        watch.monitor(gio::FileMonitorFlags::NONE, None::<&gio::Cancellable>)
                    {
                        let t = this.clone();
                        monitor.connect_changed(move |_, _, _, _| {
                            t.harvest();
                            t.render();
                        });
                        *this.0.system_monitor.borrow_mut() = Some(monitor);
                    }
                }
            }
            if this.0.config.borrow().portal_shortcut {
                this.setup_portal();
            }
            this.refresh();
            if let Some(error) = error {
                this.message(&format!(
                    "Preferences could not load; original file preserved: {error}"
                ));
            }
            this
        }
    }
    fn query(&self) -> String {
        unsafe {
            CStr::from_ptr(g::gtk_editable_get_text(self.0.input))
                .to_string_lossy()
                .into_owned()
        }
    }
    fn message(&self, s: &str) {
        eprintln!("level=info message={s:?}");
        unsafe {
            g::gtk_label_set_text(self.0.status, c(s).as_ptr());
            if !self.0.settings_status.get().is_null() {
                g::gtk_label_set_text(self.0.settings_status.get(), c(s).as_ptr());
            }
        }
    }
    pub fn command(&self, arg: &str) {
        match arg {
            "--hide" => self.hide(),
            "--toggle" if self.0.visible.get() => self.hide(),
            "--settings" => self.settings(),
            "--refresh" => self.refresh(),
            "--background" => {}
            "--quit" => self.0.app.quit(),
            _ => self.show(),
        }
    }
    fn show(&self) {
        unsafe {
            if self.0.config.borrow().clear_on_show {
                g::gtk_editable_set_text(self.0.input, c("").as_ptr())
            }
            self.render();
            self.0.visible.set(true);
            g::gtk_window_present(self.0.window);
            g::gtk_widget_grab_focus(self.0.input);
        }
    }
    fn hide(&self) {
        unsafe { g::gtk_widget_set_visible(self.0.window, 0) }
        self.0.visible.set(false);
    }
    fn theme(&self) {
        let cfg = self.0.config.borrow();
        let light = cfg.theme == "light"
            || (cfg.theme == "system"
                && self
                    .0
                    .theme_settings
                    .as_ref()
                    .is_none_or(|s| s.string("color-scheme") != "prefer-dark"));
        let css = crate::theme::stylesheet(&cfg, light);
        unsafe {
            g::gtk_widget_set_visible(self.0.settings_button, i32::from(cfg.show_settings_button));
            g::gtk_css_provider_load_from_data(self.0.provider, c(&css).as_ptr(), -1);
            g::gtk_window_set_default_size(self.0.window, cfg.width, -1);
        }
    }
    #[allow(deprecated)]
    fn refresh(&self) {
        if self.0.indexing.replace(true) {
            self.0.refresh_pending.set(true);
            return;
        }
        let (sender, receiver) = glib::MainContext::channel(glib::Priority::DEFAULT);
        std::thread::spawn(move || {
            let _ = sender.send(discover());
        });
        let t = self.clone();
        receiver.attach(None, move |entries| {
            *t.0.entries.borrow_mut() = entries;
            t.0.indexing.set(false);
            t.render();
            if t.0.refresh_pending.replace(false) {
                t.refresh();
            }
            glib::ControlFlow::Break
        });
    }
    fn render(&self) {
        unsafe {
            while !g::gtk_widget_get_first_child(self.0.list).is_null() {
                g::gtk_list_box_remove(self.0.list, g::gtk_widget_get_first_child(self.0.list));
            }
            let query = self.query();
            let q = query.trim();
            let cfg = self.0.config.borrow();
            let entries = self.0.entries.borrow();
            // Combine the launcher's own launch tally with desktop usage, when
            // enabled, so the default view reads most-used to least-used.
            let mut state = self.0.state.borrow().clone();
            if cfg.system_usage {
                state.usage = system::merge(&state.usage, &self.0.system.borrow(), &state.ignored);
            }
            let results = if tools::is_application_query(q) {
                search::search(
                    &entries,
                    q,
                    &state,
                    cfg.descriptions,
                    cfg.fuzzy,
                    cfg.results,
                )
            } else {
                vec![]
            };
            if let Some(expr) = q.strip_prefix('=') {
                let text = match tools::calculate(expr) {
                    Ok(v) => format!("{v}   ·   Enter to copy"),
                    Err(e) => e.into(),
                };
                g::gtk_list_box_append(self.0.list, label(&text));
            } else if let Some(uri) = tools::web(q) {
                g::gtk_list_box_append(self.0.list, label(&format!("Open {uri}")));
            } else {
                for &i in &results {
                    let e = &entries[i];
                    let row = g::gtk_box_new(0, 12);
                    let icon = gio::Icon::for_string(&e.icon).unwrap_or_else(|_| {
                        gio::ThemedIcon::new("application-x-executable").upcast()
                    });
                    let image = g::gtk_image_new_from_gicon(icon.as_ptr().cast());
                    g::gtk_image_set_pixel_size(image, 36);
                    g::gtk_box_append(row, image);
                    let text = g::gtk_box_new(1, 3);
                    g::gtk_widget_set_hexpand(text, 1);
                    let favorite = self.0.state.borrow().favorites.contains(&e.id);
                    let name = label(&format!("{}{}", if favorite { "★  " } else { "" }, e.name));
                    g::gtk_widget_add_css_class(name, c("result-name").as_ptr());
                    g::gtk_box_append(text, name);
                    if cfg.descriptions && !e.description.is_empty() {
                        let desc = label(&e.description);
                        g::gtk_widget_add_css_class(desc, c("description").as_ptr());
                        g::gtk_box_append(text, desc);
                    }
                    g::gtk_box_append(row, text);
                    g::gtk_list_box_append(self.0.list, row);
                }
                if results.is_empty() {
                    g::gtk_list_box_append(
                        self.0.list,
                        label(if self.0.indexing.get() {
                            "Loading installed applications…"
                        } else {
                            "No matching applications"
                        }),
                    );
                }
            }
            *self.0.results.borrow_mut() = results;
            // Size the results area to its content (two-line rows with
            // descriptions exceed the old fixed cap) instead of clipping.
            // Results beyond the cap scroll.
            const MAX_RESULTS_HEIGHT: i32 = 720;
            let mut natural: i32 = 1;
            g::gtk_widget_measure(
                self.0.list,
                1, // GTK_ORIENTATION_VERTICAL
                -1,
                std::ptr::null_mut(),
                &mut natural,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            g::gtk_scrolled_window_set_max_content_height(
                self.0.scroll,
                natural.clamp(1, MAX_RESULTS_HEIGHT),
            );
            self.0.selected.set(0);
            g::gtk_list_box_select_row(
                self.0.list,
                g::gtk_list_box_get_row_at_index(self.0.list, 0),
            );
        }
    }
    fn navigate(&self, delta: i32) {
        let n = self.0.results.borrow().len() as i32;
        if n == 0 {
            return;
        }
        let i = (self.0.selected.get() + delta).rem_euclid(n);
        self.0.selected.set(i);
        unsafe {
            g::gtk_list_box_select_row(
                self.0.list,
                g::gtk_list_box_get_row_at_index(self.0.list, i),
            );
        }
    }
    fn selected_id(&self) -> Option<String> {
        if !tools::is_application_query(&self.query()) {
            return None;
        }
        let i = *self
            .0
            .results
            .borrow()
            .get(self.0.selected.get() as usize)?;
        Some(self.0.entries.borrow().get(i)?.id.clone())
    }
    fn persist(&self) {
        if !self.0.state_writable {
            self.message(
                "History file is corrupt; original file preserved. Repair it before saving.",
            );
            return;
        }
        if let Err(e) = state::save(
            &state::directory("state").join("history.toml"),
            &*self.0.state.borrow(),
        ) {
            self.message(&format!("Could not save preferences: {e}"));
        }
    }
    fn favorite(&self) {
        if let Some(id) = self.selected_id() {
            let mut state = self.0.state.borrow_mut();
            if !state.favorites.remove(&id) {
                state.favorites.insert(id);
            }
            drop(state);
            self.persist();
            self.render();
        }
    }
    fn forget(&self) {
        if let Some(id) = self.selected_id() {
            let mut state = self.0.state.borrow_mut();
            state.usage.remove(&id);
            // Keep the entry out of desktop usage merges until it is launched
            // through Luma again.
            state.ignored.insert(id.clone());
            drop(state);
            self.persist();
            self.render();
        }
    }
    fn harvest(&self) {
        *self.0.system.borrow_mut() = system::harvest();
    }
    fn launch_context(&self) -> gio::AppLaunchContext {
        unsafe {
            from_glib_full(
                g::gdk_display_get_app_launch_context(g::gtk_widget_get_display(self.0.window))
                    .cast::<gio::ffi::GAppLaunchContext>(),
            )
        }
    }
    fn actions(&self) {
        unsafe {
            let Some(id) = self.selected_id() else {
                return;
            };
            let Some(app) = gio::DesktopAppInfo::new(&id) else {
                return;
            };
            let w = g::gtk_application_window_new(self.0.app.as_ptr().cast());
            g::gtk_widget_add_css_class(w, c("luma").as_ptr());
            g::gtk_window_set_title(w, c("Application actions").as_ptr());
            g::gtk_window_set_default_size(w, 360, 120);
            let layout = g::gtk_box_new(1, 8);
            g::gtk_window_set_child(w, layout);
            let actions = app.list_actions();
            if actions.is_empty() {
                g::gtk_box_append(layout, label("This application has no desktop actions"));
            }
            for action in actions {
                let b = button(&app.action_name(&action));
                g::gtk_box_append(layout, b);
                let a = app.clone();
                let t = self.clone();
                signal(b, "clicked", move |_| {
                    a.launch_action(&action, Some(&t.launch_context()));
                    g::gtk_window_destroy(w);
                    t.hide();
                    None
                });
            }
            g::gtk_window_present(w);
        }
    }
    fn launch(&self) {
        let query = self.query();
        let q = query.trim();
        if let Some(expr) = q.strip_prefix('=') {
            if let Ok(value) = tools::calculate(expr) {
                unsafe {
                    g::gdk_clipboard_set_text(
                        g::gtk_widget_get_clipboard(self.0.window),
                        c(&value.to_string()).as_ptr(),
                    )
                }
                self.hide();
            }
            return;
        }
        if let Some(uri) = tools::web(q) {
            match gio::AppInfo::launch_default_for_uri(&uri, Some(&self.launch_context())) {
                Ok(()) => self.hide(),
                Err(e) => self.message(&e.to_string()),
            }
            return;
        }
        if let Some(id) = self.selected_id() {
            match gio::DesktopAppInfo::new(&id)
                .ok_or_else(|| "Application was removed".to_string())
                .and_then(|a| {
                    luma_launcher::launch::desktop(
                        &a,
                        &self.launch_context(),
                        &state::directory("state").join("applications.log"),
                    )
                }) {
                Ok(()) => {
                    if self.0.config.borrow().history {
                        let mut s = self.0.state.borrow_mut();
                        s.ignored.remove(&id);
                        let u = s.usage.entry(id).or_default();
                        u.count = u.count.saturating_add(1);
                        u.last = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        while s.usage.len() > 1000 {
                            let oldest = s
                                .usage
                                .iter()
                                .min_by_key(|(_, u)| u.last)
                                .map(|(id, _)| id.clone());
                            if let Some(id) = oldest {
                                s.usage.remove(&id);
                            } else {
                                break;
                            }
                        }
                        drop(s);
                        self.persist();
                    }
                    self.hide();
                }
                Err(e) => {
                    self.message(&e);
                    self.refresh();
                }
            }
        }
    }
    fn setup_portal(&self) {
        if self.0.portal.borrow().is_some() || self.0.portal_pending.replace(true) {
            self.message("Shortcut is already registered or awaiting permission");
            return;
        }
        let toggle = self.clone();
        let closed = self.clone();
        let report = self.clone();
        let trigger = self.0.config.borrow().shortcut.clone();
        crate::portal::bind(
            &trigger,
            move |token| {
                if let Some(token) = token {
                    unsafe {
                        g::gtk_window_set_startup_id(toggle.0.window, c(&token).as_ptr());
                    }
                }
                toggle.command("--toggle");
            },
            move || {
                closed.0.portal.borrow_mut().take();
                closed.message("Shortcut session closed; register again from Settings");
            },
            move |result| {
                report.0.portal_pending.set(false);
                match result {
                    Ok(session) => {
                        *report.0.portal.borrow_mut() = Some(session);
                        report.0.config.borrow_mut().portal_shortcut = true;
                        report.message("Shortcut registered through the desktop portal");
                        if report.0.config_writable {
                            if let Err(e) = state::save(
                                &state::directory("config").join("config.toml"),
                                &*report.0.config.borrow(),
                            ) {
                                report.message(&format!("Shortcut works this session, but preferences could not save: {e}"));
                            }
                        }
                    }
                    Err(e) => report.message(&format!(
                        "Shortcut unavailable: {e}. Retry or use desktop shortcut settings."
                    )),
                }
            },
        );
    }
    fn settings(&self) {
        unsafe {
            if !self.0.settings.get().is_null() {
                g::gtk_window_present(self.0.settings.get());
                return;
            }
            let w = g::gtk_application_window_new(self.0.app.as_ptr().cast());
            g::gtk_widget_add_css_class(w, c("luma").as_ptr());
            self.0.settings.set(w);
            g::gtk_window_set_title(w, c("Luma · Settings").as_ptr());
            g::gtk_window_set_default_size(w, 560, 550);
            let layout = g::gtk_box_new(1, 8);
            g::gtk_widget_add_css_class(layout, c("palette").as_ptr());
            g::gtk_widget_add_css_class(layout, c("settings").as_ptr());
            let scroll = g::gtk_scrolled_window_new();
            g::gtk_scrolled_window_set_child(scroll, layout);
            g::gtk_window_set_child(w, scroll);
            let heading = label("Settings");
            g::gtk_widget_add_css_class(heading, c("settings-title").as_ptr());
            g::gtk_box_append(layout, heading);
            let help = label("Adjust Luma to suit your workflow. Press Enter to save a field.");
            g::gtk_label_set_ellipsize(help, 0);
            g::gtk_label_set_wrap(help, 1);
            g::gtk_widget_add_css_class(help, c("settings-help").as_ptr());
            g::gtk_box_append(layout, help);
            let section = label("Display and shortcut");
            g::gtk_widget_add_css_class(section, c("settings-section").as_ptr());
            g::gtk_box_append(layout, section);
            for (name, key, value) in [
                (
                    "Window width",
                    "width",
                    self.0.config.borrow().width.to_string(),
                ),
                (
                    "Visible results",
                    "results",
                    self.0.config.borrow().results.to_string(),
                ),
                (
                    "Font size",
                    "font_size",
                    self.0.config.borrow().font_size.to_string(),
                ),
                (
                    "Accent (#RRGGBB)",
                    "accent",
                    self.0.config.borrow().accent.clone(),
                ),
                (
                    "Portal shortcut",
                    "shortcut",
                    self.0.config.borrow().shortcut.clone(),
                ),
            ] {
                let row = g::gtk_box_new(0, 8);
                let field_label = label(name);
                g::gtk_widget_set_hexpand(field_label, 1);
                g::gtk_box_append(row, field_label);
                let entry = g::gtk_entry_new();
                g::gtk_label_set_mnemonic_widget(field_label, entry);
                g::gtk_widget_set_tooltip_text(entry, c(name).as_ptr());
                g::gtk_editable_set_text(entry, c(&value).as_ptr());
                g::gtk_box_append(row, entry);
                g::gtk_box_append(layout, row);
                let t = self.clone();
                signal(entry, "activate", move |_| {
                    let value = CStr::from_ptr(g::gtk_editable_get_text(entry)).to_string_lossy();
                    let mut cfg = t.0.config.borrow().clone();
                    let valid = match key {
                        "width" => value.parse().map(|v| cfg.width = v).is_ok(),
                        "results" => value.parse().map(|v| cfg.results = v).is_ok(),
                        "font_size" => value.parse().map(|v| cfg.font_size = v).is_ok(),
                        "accent" => {
                            cfg.accent = value.into_owned();
                            true
                        }
                        _ => {
                            cfg.shortcut = value.into_owned();
                            true
                        }
                    };
                    if valid && cfg.validate().is_ok() && t.0.config_writable {
                        if let Err(e) =
                            state::save(&state::directory("config").join("config.toml"), &cfg)
                        {
                            t.message(&e)
                        } else {
                            *t.0.config.borrow_mut() = cfg;
                            t.theme();
                            t.render();
                            t.message("Setting saved.");
                        }
                    } else {
                        t.message("Invalid setting or corrupt configuration; value not saved");
                    }
                    None
                });
            }
            for (title, key) in [
                ("Cycle theme: dark / light / system", "theme"),
                ("Toggle compact layout", "compact"),
                ("Show Settings button on launcher", "show_settings_button"),
                ("Toggle launch history", "history"),
                ("Toggle system app usage", "system_usage"),
                ("Toggle descriptions", "descriptions"),
                ("Toggle fuzzy matching", "fuzzy"),
                ("Toggle clear search on opening", "clear"),
                ("Clear launch history", "erase"),
                ("Reset personalization", "reset"),
                ("Toggle start at login", "autostart"),
                ("Register portal shortcut", "portal"),
                ("Open desktop shortcut settings", "shortcut"),
            ] {
                if let Some(section) = match key {
                    "history" => Some("Search and history"),
                    "erase" => Some("Personalization"),
                    "autostart" => Some("Shortcuts and startup"),
                    _ => None,
                } {
                    let heading = label(section);
                    g::gtk_widget_add_css_class(heading, c("settings-section").as_ptr());
                    g::gtk_box_append(layout, heading);
                }
                let active = match key {
                    "compact" => Some(self.0.config.borrow().compact),
                    "show_settings_button" => Some(self.0.config.borrow().show_settings_button),
                    "history" => Some(self.0.config.borrow().history),
                    "system_usage" => Some(self.0.config.borrow().system_usage),
                    "descriptions" => Some(self.0.config.borrow().descriptions),
                    "fuzzy" => Some(self.0.config.borrow().fuzzy),
                    "clear" => Some(self.0.config.borrow().clear_on_show),
                    _ => None,
                };
                let button = if let Some(active) = active {
                    let control = g::gtk_check_button_new_with_label(
                        c(title.trim_start_matches("Toggle ")).as_ptr(),
                    );
                    g::gtk_check_button_set_active(control, i32::from(active));
                    control
                } else {
                    button(&setting_text(title, key, &self.0.config.borrow()))
                };
                g::gtk_box_append(layout, button);
                let t = self.clone();
                let restoring = Cell::new(false);
                signal(
                    button,
                    if active.is_some() {
                        "toggled"
                    } else {
                        "clicked"
                    },
                    move |_| {
                        if restoring.get() {
                            return None;
                        }
                        let previous = t.0.config.borrow().clone();
                        let mut cfg = t.0.config.borrow_mut();
                        match key {
                            "theme" => {
                                cfg.theme = match cfg.theme.as_str() {
                                    "dark" => "light",
                                    "light" => "system",
                                    _ => "dark",
                                }
                                .into()
                            }
                            "compact" => cfg.compact = !cfg.compact,
                            "show_settings_button" => {
                                cfg.show_settings_button = !cfg.show_settings_button
                            }
                            "history" => cfg.history = !cfg.history,
                            "system_usage" => cfg.system_usage = !cfg.system_usage,
                            "descriptions" => cfg.descriptions = !cfg.descriptions,
                            "fuzzy" => cfg.fuzzy = !cfg.fuzzy,
                            "clear" => cfg.clear_on_show = !cfg.clear_on_show,
                            "erase" => t.0.state.borrow_mut().usage.clear(),
                            "reset" => *t.0.state.borrow_mut() = State::default(),
                            "portal" => {
                                drop(cfg);
                                t.setup_portal();
                                return None;
                            }
                            "autostart" => {
                                if let Err(e) = autostart() {
                                    t.message(&e)
                                }
                            }
                            "shortcut" => {
                                let desktop =
                                    std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
                                let command = if desktop.contains("KDE") {
                                    ("systemsettings", "kcm_keys")
                                } else {
                                    ("gnome-control-center", "keyboard")
                                };
                                if let Err(e) =
                                    std::process::Command::new(command.0).arg(command.1).spawn()
                                {
                                    t.message(&e.to_string())
                                }
                                t.message("Create a shortcut for luma-launcher --toggle; resolve Alt+Space conflicts in desktop Settings.");
                            }
                            _ => {}
                        }
                        if active.is_none() {
                            gtk_button_set_label(
                                button,
                                c(&setting_text(title, key, &cfg)).as_ptr(),
                            );
                        }
                        let saved = if t.0.config_writable {
                            state::save(&state::directory("config").join("config.toml"), &*cfg)
                        } else {
                            Err("Corrupt configuration preserved; repair it before saving.".into())
                        };
                        drop(cfg);
                        if let Err(e) = saved {
                            if active.is_some() {
                                *t.0.config.borrow_mut() = previous;
                                restoring.set(true);
                                let checked = object(button).property::<bool>("active");
                                g::gtk_check_button_set_active(button, i32::from(!checked));
                                restoring.set(false);
                            }
                            t.message(&e)
                        }
                        t.persist();
                        t.theme();
                        t.render();
                        None
                    },
                );
            }
            let tips = label("Ctrl+D: pin or unpin    Ctrl+Delete: forget an app");
            g::gtk_widget_add_css_class(tips, c("settings-help").as_ptr());
            g::gtk_box_append(layout, tips);
            let status = label("Press Enter in a setting field to save it.");
            g::gtk_label_set_ellipsize(status, 0);
            g::gtk_label_set_wrap(status, 1);
            g::gtk_widget_add_css_class(status, c("settings-help").as_ptr());
            self.0.settings_status.set(status);
            g::gtk_box_append(layout, status);
            let t = self.clone();
            signal(w, "close-request", move |_| {
                t.0.settings.set(std::ptr::null_mut());
                t.0.settings_status.set(std::ptr::null_mut());
                Some(false.to_value())
            });
            g::gtk_window_present(w);
        }
    }
}
unsafe extern "C" {
    fn gtk_button_set_label(w: Widget, s: *const std::ffi::c_char);
    fn gtk_button_new_with_label(s: *const std::ffi::c_char) -> Widget;
}
unsafe fn button(s: &str) -> Widget {
    gtk_button_new_with_label(c(s).as_ptr())
}
pub fn diagnostics() {
    let usage_source = system::backend()
        .map(|(b, p)| format!("{} ({})", b.label(), p.display()))
        .unwrap_or_else(|| "unavailable".into());
    println!("Luma {}\nDesktop: {}\nSession: {}\nGTK: {}.{}\nRust: {}\nConfiguration: {}\nShortcut backend: portal registration on explicit setup; desktop-managed fallback\nSession bus: {}\nUsage source: {}\nInstalled applications: {}",env!("CARGO_PKG_VERSION"),std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),unsafe{g::gtk_get_major_version()},unsafe{g::gtk_get_minor_version()},env!("LUMA_RUSTC"),state::directory("config").display(),if gio::bus_get_sync(gio::BusType::Session,None::<&gio::Cancellable>).is_ok(){"accessible"}else{"unavailable"},usage_source,discover().len());
}

fn system_settings() -> Option<gio::Settings> {
    let source = gio::SettingsSchemaSource::default()?;
    let schema = source.lookup("org.gnome.desktop.interface", true)?;
    if !schema.has_key("color-scheme") {
        return None;
    }
    Some(gio::Settings::new_full(
        &schema,
        None::<&gio::SettingsBackend>,
        None,
    ))
}
fn autostart() -> Result<(), String> {
    let root = state::directory("config");
    let dir = root
        .parent()
        .ok_or("No config directory")?
        .join("autostart");
    let path = dir.join("io.github.luma_launcher.LumaLauncher.desktop");
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    } else {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        state::atomic_write(&path, state::autostart_entry(&executable)?.as_bytes())
    }
}

fn setting_text(title: &str, key: &str, cfg: &Config) -> String {
    let state = match key {
        "theme" => return format!("Theme: {} (click to cycle)", cfg.theme),
        "compact" => cfg.compact,
        "history" => cfg.history,
        "system_usage" => cfg.system_usage,
        "descriptions" => cfg.descriptions,
        "fuzzy" => cfg.fuzzy,
        "clear" => cfg.clear_on_show,
        _ => return title.to_owned(),
    };
    format!(
        "{}: {}",
        title.trim_start_matches("Toggle "),
        if state { "on" } else { "off" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a graphical desktop session"]
    fn focus_dismissal_and_settings_save_rollback() {
        unsafe fn checkbox(widget: Widget) -> Option<Widget> {
            if object(widget).type_().name() == "GtkCheckButton" {
                return Some(widget);
            }
            let mut child = g::gtk_widget_get_first_child(widget);
            while !child.is_null() {
                if let Some(found) = checkbox(child) {
                    return Some(found);
                }
                child = g::gtk_widget_get_next_sibling(child);
            }
            None
        }
        let root = std::env::temp_dir().join(format!("luma-focus-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &root);
        std::env::set_var("XDG_STATE_HOME", &root);
        unsafe {
            let app: gio::Application = from_glib_full(
                g::gtk_application_new(
                    c("io.github.luma_launcher.FocusTest").as_ptr(),
                    gio::ApplicationFlags::NON_UNIQUE.into_glib(),
                )
                .cast::<gio::ffi::GApplication>(),
            );
            app.register(None::<&gio::Cancellable>).unwrap();
            let launcher = Launcher::new(&app);
            let window = object(launcher.0.window);
            launcher.command("--show");
            assert!(window.property::<bool>("visible"));

            // No compositor events have been dispatched, so the window is inactive.
            assert!(!window.property::<bool>("is-active"));
            window.notify("is-active");
            assert!(!window.property::<bool>("visible"));
            assert!(!launcher.0.visible.get());

            launcher.command("--toggle");
            assert!(window.property::<bool>("visible"));
            assert!(launcher.0.visible.get());
            launcher.hide();

            launcher.command("--settings");
            let control = checkbox(launcher.0.settings.get()).expect("native settings checkbox");
            assert!(!object(control).property::<bool>("active"));
            let directory = root.join("luma-launcher");
            std::fs::write(&directory, "blocks configuration writes").unwrap();
            g::gtk_check_button_set_active(control, 1);
            assert!(!object(control).property::<bool>("active"));
            assert!(!launcher.0.config.borrow().compact);

            std::fs::remove_file(&directory).unwrap();
            g::gtk_check_button_set_active(control, 1);
            assert!(object(control).property::<bool>("active"));
            assert!(launcher.0.config.borrow().compact);
            assert!(
                state::load::<Config>(&directory.join("config.toml"))
                    .unwrap()
                    .compact
            );
            g::gtk_window_destroy(launcher.0.settings.get());
            g::gtk_window_destroy(launcher.0.window);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn settings_shortcut(key: u32, ctrl: bool) -> bool {
    // Accept the plus key with or without Shift, including the numeric keypad.
    ctrl && matches!(key, 0x2b | 0x2c | 0x3d | 0xffab)
}

unsafe fn key_controller() -> Widget {
    let controller = g::gtk_event_controller_key_new();
    // Capture before GtkEntry consumes Ctrl+J/K, Ctrl+D, or Ctrl+Enter.
    g::gtk_event_controller_set_propagation_phase(controller, 1);
    controller
}
#[cfg(test)]
mod keyboard_tests {
    use super::*;
    #[test]
    fn settings_accepts_comma_and_plus_only_with_control() {
        for key in [u32::from(b','), u32::from(b'+'), u32::from(b'='), 0xffab] {
            assert!(settings_shortcut(key, true), "Ctrl with key {key:#x}");
            assert!(!settings_shortcut(key, false), "plain key {key:#x}");
        }
        assert!(!settings_shortcut(u32::from(b'a'), true));
    }

    #[test]
    fn shortcuts_run_before_text_editing() {
        unsafe {
            let controller = key_controller();
            let obj: glib::Object = from_glib_full(controller.cast::<glib::gobject_ffi::GObject>());
            let phase = obj.property_value("propagation-phase");
            assert_eq!(
                glib::gobject_ffi::g_value_get_enum(phase.to_glib_none().0),
                1
            );
        }
    }
}
