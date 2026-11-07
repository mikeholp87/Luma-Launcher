use glib::clone;
use gtk4::{self as gtk, gdk, pango, prelude::*};
use luma_launcher::{
    search::{self, Entry},
    state::{self, Config, State, Usage},
    system, tools,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};

fn label(s: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(s)
        .xalign(0.)
        .ellipsize(pango::EllipsizeMode::End)
        .build()
}
/// Label that wraps instead of ellipsizing, for status and help text.
fn wrapped(s: &str) -> gtk::Label {
    let l = label(s);
    l.set_ellipsize(pango::EllipsizeMode::None);
    l.set_wrap(true);
    l
}
fn section(layout: &gtk::Box, text: &str) {
    let heading = label(text);
    heading.add_css_class("settings-section");
    layout.append(&heading);
}
/// Informational list row: neither selectable nor activatable.
fn note(list: &gtk::ListBox, text: &str) {
    list.append(
        &gtk::ListBoxRow::builder()
            .child(&label(text))
            .selectable(false)
            .activatable(false)
            .build(),
    );
}

#[derive(Debug, PartialEq)]
enum Shortcut {
    Actions,
    Escape,
    Next,
    Prev,
    Jump(usize),
    Settings,
    Favorite,
    Forget,
}
/// Key bindings, resolved before the entry can consume them. `to_lower`
/// keeps letter shortcuts working under Caps Lock and Shift.
fn shortcut(key: gdk::Key, ctrl: bool) -> Option<Shortcut> {
    use gdk::Key as K;
    Some(match key {
        K::Escape => Shortcut::Escape,
        K::Down => Shortcut::Next,
        K::Up => Shortcut::Prev,
        _ if !ctrl => return None,
        K::Return => Shortcut::Actions,
        K::Delete => Shortcut::Forget,
        // Plus with or without Shift, including the numeric keypad.
        K::plus | K::comma | K::equal | K::KP_Add => Shortcut::Settings,
        _ => match key.to_lower() {
            K::j | K::n => Shortcut::Next,
            K::k | K::p => Shortcut::Prev,
            K::d => Shortcut::Favorite,
            other => match other.to_unicode() {
                Some(digit @ '1'..='9') => Shortcut::Jump(digit as usize - '1' as usize),
                _ => return None,
            },
        },
    })
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
            let d = gio_unix::DesktopAppInfo::new(&id);
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

/// Shared handle to the launcher. Closures stored on long-lived objects hold
/// the weak form (`clone!(#[weak] …)`) so widgets never keep it alive.
#[derive(Clone)]
pub struct Launcher(Rc<Inner>);
pub struct WeakLauncher(Weak<Inner>);
impl glib::clone::Downgrade for Launcher {
    type Weak = WeakLauncher;
    fn downgrade(&self) -> WeakLauncher {
        WeakLauncher(Rc::downgrade(&self.0))
    }
}
impl glib::clone::Upgrade for WeakLauncher {
    type Strong = Launcher;
    fn upgrade(&self) -> Option<Launcher> {
        self.0.upgrade().map(Launcher)
    }
}

struct Inner {
    app: gtk::Application,
    _hold: gio::ApplicationHoldGuard,
    window: gtk::ApplicationWindow,
    input: gtk::Entry,
    settings_button: gtk::Button,
    list: gtk::ListBox,
    scroll: gtk::ScrolledWindow,
    status: gtk::Label,
    settings_status: RefCell<Option<gtk::Label>>,
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
    harvest_pending: Cell<bool>,
    usage_view: RefCell<Option<Rc<State>>>,
    provider: gtk::CssProvider,
    settings: RefCell<Option<gtk::ApplicationWindow>>,
    indexing: Cell<bool>,
    refresh_pending: Cell<bool>,
    monitor: gio::AppInfoMonitor,
    theme_settings: Option<gio::Settings>,
    portal_pending: Cell<bool>,
    portal: RefCell<Option<crate::portal::Session>>,
}

fn config_path() -> std::path::PathBuf {
    state::directory("config").join("config.toml")
}
fn log_path() -> std::path::PathBuf {
    state::directory("state").join("applications.log")
}

/// One row of the Settings list. Widgets are held weakly: the handler that
/// owns this lives on the widget itself.
struct Setting {
    key: &'static str,
    title: &'static str,
    control: Control,
    restoring: Cell<bool>,
    armed: Cell<bool>,
}
enum Control {
    Check(glib::WeakRef<gtk::CheckButton>),
    Push(glib::WeakRef<gtk::Button>),
}
impl Setting {
    fn set_label(&self, text: &str) {
        if let Control::Push(button) = &self.control {
            if let Some(button) = button.upgrade() {
                button.set_label(text);
            }
        }
    }
}

impl Launcher {
    pub fn new(app: &gtk::Application) -> Self {
        let config = state::load::<Config>(&config_path()).and_then(|c| {
            c.validate()?;
            Ok(c)
        });
        let history = state::load::<State>(&state::directory("state").join("history.toml"));
        let error = config.as_ref().err().or(history.as_ref().err()).cloned();

        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Luma Launcher")
            .decorated(false)
            .css_classes(["luma", "launcher"])
            .build();
        let layout = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .css_classes(["palette"])
            .build();
        window.set_child(Some(&layout));
        let input = gtk::Entry::builder()
            .css_classes(["search"])
            .max_length(512)
            .placeholder_text("Search applications...")
            .hexpand(true)
            .build();
        input.set_icon_from_icon_name(
            gtk::EntryIconPosition::Primary,
            Some("system-search-symbolic"),
        );
        let search_bar = gtk::Box::builder()
            .spacing(8)
            .css_classes(["search-bar"])
            .build();
        search_bar.append(&input);
        let settings_button = gtk::Button::builder()
            .label("Settings")
            .tooltip_text("Open Settings (Ctrl+, or Ctrl++)")
            .build();
        search_bar.append(&settings_button);
        layout.append(&search_bar);
        let list = gtk::ListBox::new();
        let scroll = gtk::ScrolledWindow::builder()
            .child(&list)
            .max_content_height(380)
            .propagate_natural_height(true)
            .build();
        layout.append(&scroll);
        let status = wrapped("↑↓ Navigate    Enter Open    Ctrl+1–9 Jump    Esc Clear / Close");
        status.add_css_class("footer");
        layout.append(&status);
        let provider = gtk::CssProvider::new();
        gtk::style_context_add_provider_for_display(
            &WidgetExt::display(&window),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let this = Self(Rc::new(Inner {
            app: app.clone(),
            _hold: app.hold(),
            window: window.clone(),
            input: input.clone(),
            settings_button: settings_button.clone(),
            list: list.clone(),
            scroll,
            status,
            settings_status: RefCell::new(None),
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
            harvest_pending: Cell::new(false),
            usage_view: RefCell::new(None),
            provider,
            settings: RefCell::new(None),
            indexing: Cell::new(false),
            refresh_pending: Cell::new(false),
            monitor: gio::AppInfoMonitor::get(),
            theme_settings: system_settings(),
            portal_pending: Cell::new(false),
            portal: RefCell::new(None),
        }));
        if let Some(settings) = &this.0.theme_settings {
            settings.connect_changed(
                Some("color-scheme"),
                clone!(
                    #[weak(rename_to = t)]
                    this,
                    move |_, _| t.theme()
                ),
            );
        }
        this.theme();
        settings_button.connect_clicked(clone!(
            #[weak(rename_to = t)]
            this,
            move |_| t.settings()
        ));
        input.connect_changed(clone!(
            #[weak(rename_to = t)]
            this,
            move |_| t.render()
        ));
        input.connect_activate(clone!(
            #[weak(rename_to = t)]
            this,
            move |_| t.launch()
        ));
        list.connect_row_activated(clone!(
            #[weak(rename_to = t)]
            this,
            move |_, row| {
                t.0.selected.set(row.index());
                t.launch();
            }
        ));
        list.connect_row_selected(clone!(
            #[weak(rename_to = t)]
            this,
            move |_, row| {
                if let Some(row) = row {
                    t.0.selected.set(row.index());
                }
            }
        ));
        window.connect_close_request(clone!(
            #[weak(rename_to = t)]
            this,
            #[upgrade_or]
            glib::Propagation::Stop,
            move |_| {
                t.hide();
                glib::Propagation::Stop
            }
        ));
        window.connect_is_active_notify(clone!(
            #[weak(rename_to = t)]
            this,
            move |window| {
                if !window.is_active() {
                    t.hide();
                }
            }
        ));
        // Capture before the entry consumes Ctrl+J/K, Ctrl+D, or Ctrl+Enter.
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        keys.connect_key_pressed(clone!(
            #[weak(rename_to = t)]
            this,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, mods| {
                let ctrl = mods.contains(gdk::ModifierType::CONTROL_MASK);
                let Some(action) = shortcut(key, ctrl) else {
                    return glib::Propagation::Proceed;
                };
                match action {
                    Shortcut::Actions => t.actions(),
                    Shortcut::Escape if t.query().is_empty() => t.hide(),
                    Shortcut::Escape => t.0.input.set_text(""),
                    Shortcut::Next => t.navigate(1),
                    Shortcut::Prev => t.navigate(-1),
                    Shortcut::Jump(index) => t.quick_launch(index),
                    Shortcut::Settings => t.settings(),
                    Shortcut::Favorite => t.favorite(),
                    Shortcut::Forget => t.forget(),
                }
                glib::Propagation::Stop
            }
        ));
        window.add_controller(keys);
        this.0.monitor.connect_changed(clone!(
            #[weak(rename_to = t)]
            this,
            move |_| t.refresh()
        ));
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
                    monitor.connect_changed(clone!(
                        #[weak(rename_to = t)]
                        this,
                        move |_, _, _, _| t.schedule_harvest()
                    ));
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
    fn query(&self) -> String {
        self.0.input.text().to_string()
    }
    fn message(&self, s: &str) {
        eprintln!("level=info message={s:?}");
        self.0.status.set_text(s);
        if let Some(status) = self.0.settings_status.borrow().as_ref() {
            status.set_text(s);
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
        if self.0.config.borrow().clear_on_show {
            self.0.input.set_text("");
        }
        self.render();
        self.0.visible.set(true);
        self.0.window.present();
        self.0.input.grab_focus();
    }
    fn hide(&self) {
        self.0.window.set_visible(false);
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
        self.0.settings_button.set_visible(cfg.show_settings_button);
        self.0
            .provider
            .load_from_data(&crate::theme::stylesheet(&cfg, light));
        self.0.window.set_default_size(cfg.width, -1);
    }
    fn refresh(&self) {
        if self.0.indexing.replace(true) {
            self.0.refresh_pending.set(true);
            return;
        }
        let t = self.clone();
        glib::spawn_future_local(async move {
            // A panicking worker must not leave `indexing` stuck on.
            let entries = gio::spawn_blocking(discover).await.unwrap_or_default();
            *t.0.entries.borrow_mut() = entries;
            t.0.indexing.set(false);
            t.invalidate();
            if t.0.visible.get() {
                t.render();
            }
            if t.0.refresh_pending.replace(false) {
                t.refresh();
            }
        });
    }
    fn render(&self) {
        let list = &self.0.list;
        while let Some(child) = list.first_child() {
            list.remove(&child);
        }
        let query = self.query();
        let q = query.trim();
        let cfg = self.0.config.borrow();
        let entries = self.0.entries.borrow();
        let state = self.state_view();
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
            match tools::calculate(expr) {
                Ok(v) => list.append(&label(&format!(
                    "{}   ·   Enter to copy",
                    tools::format_number(v)
                ))),
                Err(e) => note(list, e),
            }
        } else if let Some(uri) = tools::web(q) {
            list.append(&label(&format!("Open {uri}")));
        } else {
            for &i in &results {
                let e = &entries[i];
                let row = gtk::Box::builder().spacing(12).build();
                let icon = gio::Icon::for_string(&e.icon)
                    .unwrap_or_else(|_| gio::ThemedIcon::new("application-x-executable").upcast());
                row.append(&gtk::Image::builder().gicon(&icon).pixel_size(36).build());
                let text = gtk::Box::builder()
                    .orientation(gtk::Orientation::Vertical)
                    .spacing(3)
                    .hexpand(true)
                    .build();
                let favorite = self.0.state.borrow().favorites.contains(&e.id);
                let name = label(&format!("{}{}", if favorite { "★  " } else { "" }, e.name));
                name.add_css_class("result-name");
                text.append(&name);
                if cfg.descriptions && !e.description.is_empty() {
                    let desc = label(&e.description);
                    desc.add_css_class("description");
                    text.append(&desc);
                }
                row.append(&text);
                list.append(&row);
            }
            if results.is_empty() {
                note(
                    list,
                    if self.0.indexing.get() {
                        "Loading installed applications…"
                    } else {
                        "No matching applications"
                    },
                );
            }
        }
        *self.0.results.borrow_mut() = results;
        // Size the results area to its content (two-line rows with
        // descriptions exceed a fixed cap) instead of clipping. Results
        // beyond the cap scroll.
        const MAX_RESULTS_HEIGHT: i32 = 720;
        let (_, natural, _, _) = list.measure(gtk::Orientation::Vertical, -1);
        self.0
            .scroll
            .set_max_content_height(natural.clamp(1, MAX_RESULTS_HEIGHT));
        self.0.selected.set(0);
        list.select_row(list.row_at_index(0).as_ref());
    }
    fn navigate(&self, delta: i32) {
        let n = self.0.results.borrow().len() as i32;
        if n == 0 {
            return;
        }
        let i = (self.0.selected.get() + delta).rem_euclid(n);
        self.0.selected.set(i);
        self.0.list.select_row(self.0.list.row_at_index(i).as_ref());
    }
    fn selected_id(&self) -> Option<String> {
        if !tools::is_application_query(&self.query()) {
            return None;
        }
        let i = *self
            .0
            .results
            .borrow()
            .get(usize::try_from(self.0.selected.get()).ok()?)?;
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
            self.invalidate();
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
            self.invalidate();
            self.persist();
            self.render();
        }
    }
    /// Read desktop usage off the UI thread (KDE spawns `sqlite3`).
    fn harvest(&self) {
        let t = self.clone();
        glib::spawn_future_local(async move {
            let usage = gio::spawn_blocking(system::harvest)
                .await
                .unwrap_or_default();
            *t.0.system.borrow_mut() = usage;
            t.invalidate();
            // Ranking only matters on screen, and re-rendering mid-typing
            // would reset the selection.
            if t.0.visible.get() && t.query().is_empty() {
                t.render();
            }
        });
    }
    /// Directory monitors fire per write (SQLite WAL is continuous), so
    /// coalesce a burst into one harvest.
    fn schedule_harvest(&self) {
        if self.0.harvest_pending.replace(true) {
            return;
        }
        let t = self.clone();
        let _ = glib::timeout_add_local_once(std::time::Duration::from_secs(2), move || {
            t.0.harvest_pending.set(false);
            t.harvest();
        });
    }
    /// Launch tally merged with desktop usage, cached until an input changes.
    fn state_view(&self) -> Rc<State> {
        if let Some(view) = self.0.usage_view.borrow().as_ref() {
            return view.clone();
        }
        let mut state = self.0.state.borrow().clone();
        if self.0.config.borrow().system_usage {
            state.usage = system::merge(&state.usage, &self.0.system.borrow(), &state.ignored);
        }
        let view = Rc::new(state);
        *self.0.usage_view.borrow_mut() = Some(view.clone());
        view
    }
    fn invalidate(&self) {
        *self.0.usage_view.borrow_mut() = None;
    }
    fn quick_launch(&self, index: usize) {
        if index < self.0.results.borrow().len() {
            self.0.selected.set(index as i32);
            self.0
                .list
                .select_row(self.0.list.row_at_index(index as i32).as_ref());
            self.launch();
        }
    }
    fn launch_context(&self) -> gio::AppLaunchContext {
        WidgetExt::display(&self.0.window)
            .app_launch_context()
            .upcast()
    }
    fn actions(&self) {
        let Some(id) = self.selected_id() else {
            return;
        };
        let Some(app) = gio_unix::DesktopAppInfo::new(&id) else {
            return;
        };
        let w = gtk::ApplicationWindow::builder()
            .application(&self.0.app)
            .title("Application actions")
            .default_width(360)
            .default_height(120)
            .css_classes(["luma"])
            .build();
        let layout = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .build();
        w.set_child(Some(&layout));
        let actions = app.list_actions();
        if actions.is_empty() {
            layout.append(&label("This application has no desktop actions"));
        }
        let mut first: Option<gtk::Button> = None;
        for action in actions {
            let b = gtk::Button::with_label(&app.action_name(&action));
            layout.append(&b);
            first.get_or_insert_with(|| b.clone());
            b.connect_clicked(clone!(
                #[strong(rename_to = a)]
                app,
                #[weak(rename_to = t)]
                self,
                #[weak]
                w,
                move |_| {
                    if let Err(e) =
                        luma_launcher::launch::action(&a, &action, &t.launch_context(), &log_path())
                    {
                        t.message(&e);
                    }
                    w.destroy();
                    t.hide();
                }
            ));
        }
        let keys = gtk::EventControllerKey::new();
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        keys.connect_key_pressed(clone!(
            #[weak]
            w,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    w.destroy();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        w.add_controller(keys);
        w.present();
        if let Some(first) = first {
            first.grab_focus();
        }
    }
    fn launch(&self) {
        let query = self.query();
        let q = query.trim();
        if let Some(expr) = q.strip_prefix('=') {
            if let Ok(value) = tools::calculate(expr) {
                WidgetExt::clipboard(&self.0.window).set_text(&tools::format_number(value));
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
        let Some(id) = self.selected_id() else {
            return;
        };
        match gio_unix::DesktopAppInfo::new(&id)
            .ok_or_else(|| "Application was removed".to_string())
            .and_then(|a| luma_launcher::launch::desktop(&a, &self.launch_context(), &log_path()))
        {
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
                    self.invalidate();
                    // fsync must not delay dismissing the window; idle
                    // priority runs after the hide has been redrawn.
                    let t = self.clone();
                    glib::idle_add_local_once(move || t.persist());
                }
                self.hide();
            }
            Err(e) => {
                self.message(&e);
                self.refresh();
            }
        }
    }
    fn setup_portal(&self) {
        if self.0.portal.borrow().is_some() || self.0.portal_pending.replace(true) {
            self.message("Shortcut is already registered or awaiting permission");
            return;
        }
        let trigger = self.0.config.borrow().shortcut.clone();
        crate::portal::bind(
            &trigger,
            clone!(
                #[weak(rename_to = t)]
                self,
                move |token| {
                    if let Some(token) = token {
                        t.0.window.set_startup_id(&token);
                    }
                    t.command("--toggle");
                }
            ),
            clone!(
                #[weak(rename_to = t)]
                self,
                move || {
                    t.0.portal.borrow_mut().take();
                    t.message("Shortcut session closed; register again from Settings");
                }
            ),
            clone!(
                #[strong(rename_to = t)]
                self,
                move |result| {
                    t.0.portal_pending.set(false);
                    match result {
                        Ok(session) => {
                            *t.0.portal.borrow_mut() = Some(session);
                            t.0.config.borrow_mut().portal_shortcut = true;
                            t.message("Shortcut registered through the desktop portal");
                            if t.0.config_writable {
                                if let Err(e) = state::save(&config_path(), &*t.0.config.borrow()) {
                                    t.message(&format!(
                                        "Shortcut works this session, but preferences could not save: {e}"
                                    ));
                                }
                            }
                        }
                        Err(e) => t.message(&format!(
                            "Shortcut unavailable: {e}. Retry or use desktop shortcut settings."
                        )),
                    }
                }
            ),
        );
    }
    fn settings(&self) {
        if let Some(open) = self.0.settings.borrow().as_ref() {
            open.present();
            return;
        }
        let w = gtk::ApplicationWindow::builder()
            .application(&self.0.app)
            .title("Luma · Settings")
            .default_width(560)
            .default_height(550)
            .css_classes(["luma"])
            .build();
        let layout = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .css_classes(["palette", "settings"])
            .build();
        w.set_child(Some(&gtk::ScrolledWindow::builder().child(&layout).build()));
        let heading = label("Settings");
        heading.add_css_class("settings-title");
        layout.append(&heading);
        let help = wrapped("Adjust Luma to suit your workflow. Press Enter to save a field.");
        help.add_css_class("settings-help");
        layout.append(&help);
        section(&layout, "Display and shortcut");
        let fields = {
            let cfg = self.0.config.borrow();
            [
                ("Window width", "width", cfg.width.to_string()),
                ("Visible results", "results", cfg.results.to_string()),
                ("Font size", "font_size", cfg.font_size.to_string()),
                ("Accent (#RRGGBB)", "accent", cfg.accent.clone()),
                ("Portal shortcut", "shortcut", cfg.shortcut.clone()),
            ]
        };
        for (name, key, value) in fields {
            let row = gtk::Box::builder().spacing(8).build();
            let field_label = label(name);
            field_label.set_hexpand(true);
            row.append(&field_label);
            let entry = gtk::Entry::builder()
                .tooltip_text(name)
                .text(&value)
                .build();
            field_label.set_mnemonic_widget(Some(&entry));
            row.append(&entry);
            layout.append(&row);
            entry.connect_activate(clone!(
                #[weak(rename_to = t)]
                self,
                move |entry| t.field_changed(key, &entry.text())
            ));
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
            match key {
                "history" => section(&layout, "Search and history"),
                "erase" => section(&layout, "Personalization"),
                "autostart" => section(&layout, "Shortcuts and startup"),
                _ => {}
            }
            let active = {
                let cfg = self.0.config.borrow();
                match key {
                    "compact" => Some(cfg.compact),
                    "show_settings_button" => Some(cfg.show_settings_button),
                    "history" => Some(cfg.history),
                    "system_usage" => Some(cfg.system_usage),
                    "descriptions" => Some(cfg.descriptions),
                    "fuzzy" => Some(cfg.fuzzy),
                    "clear" => Some(cfg.clear_on_show),
                    _ => None,
                }
            };
            if let Some(active) = active {
                let check = gtk::CheckButton::with_label(title.trim_start_matches("Toggle "));
                check.set_active(active);
                layout.append(&check);
                let setting = Setting {
                    key,
                    title,
                    control: Control::Check(check.downgrade()),
                    restoring: Cell::new(false),
                    armed: Cell::new(false),
                };
                // Connected after set_active so building the row is not a toggle.
                check.connect_toggled(clone!(
                    #[weak(rename_to = t)]
                    self,
                    move |_| t.setting_changed(&setting)
                ));
            } else {
                let button =
                    gtk::Button::with_label(&setting_text(title, key, &self.0.config.borrow()));
                layout.append(&button);
                let setting = Setting {
                    key,
                    title,
                    control: Control::Push(button.downgrade()),
                    restoring: Cell::new(false),
                    armed: Cell::new(false),
                };
                button.connect_clicked(clone!(
                    #[weak(rename_to = t)]
                    self,
                    move |_| t.setting_changed(&setting)
                ));
            }
        }
        let tips = label("Ctrl+D: pin or unpin    Ctrl+Delete: forget an app");
        tips.add_css_class("settings-help");
        layout.append(&tips);
        let status = wrapped("Press Enter in a setting field to save it.");
        status.add_css_class("settings-help");
        layout.append(&status);
        *self.0.settings_status.borrow_mut() = Some(status);
        w.connect_close_request(clone!(
            #[weak(rename_to = t)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                *t.0.settings.borrow_mut() = None;
                *t.0.settings_status.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        *self.0.settings.borrow_mut() = Some(w.clone());
        w.present();
    }
    /// A Settings text field was committed with Enter.
    fn field_changed(&self, key: &str, value: &str) {
        let mut cfg = self.0.config.borrow().clone();
        let valid = match key {
            "width" => value.parse().map(|v| cfg.width = v).is_ok(),
            "results" => value.parse().map(|v| cfg.results = v).is_ok(),
            "font_size" => value.parse().map(|v| cfg.font_size = v).is_ok(),
            "accent" => {
                cfg.accent = value.to_owned();
                true
            }
            _ => {
                cfg.shortcut = value.to_owned();
                true
            }
        };
        if valid && cfg.validate().is_ok() && self.0.config_writable {
            if let Err(e) = state::save(&config_path(), &cfg) {
                self.message(&e)
            } else {
                *self.0.config.borrow_mut() = cfg;
                self.theme();
                self.render();
                self.message("Setting saved.");
            }
        } else {
            self.message("Invalid setting or corrupt configuration; value not saved");
        }
    }
    /// A Settings toggle or button was activated.
    fn setting_changed(&self, s: &Setting) {
        if s.restoring.get() {
            return;
        }
        let (key, title) = (s.key, s.title);
        let destructive = matches!(key, "erase" | "reset");
        if destructive && !s.armed.replace(true) {
            s.set_label(&format!("Click again to confirm: {title}"));
            self.message("Click again to confirm; this cannot be undone.");
            return;
        }
        s.armed.set(false);
        let previous = self.0.config.borrow().clone();
        let mut cfg = self.0.config.borrow_mut();
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
            "show_settings_button" => cfg.show_settings_button = !cfg.show_settings_button,
            "history" => cfg.history = !cfg.history,
            "system_usage" => cfg.system_usage = !cfg.system_usage,
            "descriptions" => cfg.descriptions = !cfg.descriptions,
            "fuzzy" => cfg.fuzzy = !cfg.fuzzy,
            "clear" => cfg.clear_on_show = !cfg.clear_on_show,
            "erase" => self.0.state.borrow_mut().usage.clear(),
            "reset" => *self.0.state.borrow_mut() = State::default(),
            "portal" => {
                drop(cfg);
                self.setup_portal();
                return;
            }
            "autostart" => {
                if let Err(e) = autostart() {
                    self.message(&e)
                }
            }
            "shortcut" => {
                let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
                let command = if desktop.contains("KDE") {
                    ("systemsettings", "kcm_keys")
                } else {
                    ("gnome-control-center", "keyboard")
                };
                if let Err(e) = std::process::Command::new(command.0).arg(command.1).spawn() {
                    self.message(&e.to_string())
                }
                self.message("Create a shortcut for luma-launcher --toggle; resolve Alt+Space conflicts in desktop Settings.");
            }
            _ => {}
        }
        s.set_label(&setting_text(title, key, &cfg));
        let touches_config = !matches!(key, "erase" | "reset" | "shortcut" | "autostart");
        let saved = if !touches_config {
            Ok(())
        } else if self.0.config_writable {
            state::save(&config_path(), &*cfg)
        } else {
            Err("Corrupt configuration preserved; repair it before saving.".into())
        };
        drop(cfg);
        if let Err(e) = saved {
            *self.0.config.borrow_mut() = previous;
            match &s.control {
                Control::Check(check) => {
                    if let Some(check) = check.upgrade() {
                        s.restoring.set(true);
                        check.set_active(!check.is_active());
                        s.restoring.set(false);
                    }
                }
                Control::Push(_) => {
                    s.set_label(&setting_text(title, key, &self.0.config.borrow()));
                }
            }
            self.message(&e)
        }
        if destructive {
            self.persist();
        }
        self.invalidate();
        self.theme();
        self.render();
    }
}

fn autostart_path() -> Result<std::path::PathBuf, String> {
    let root = state::directory("config");
    Ok(root
        .parent()
        .ok_or("No config directory")?
        .join("autostart")
        .join("io.github.luma_launcher.LumaLauncher.desktop"))
}
fn autostart() -> Result<(), String> {
    let path = autostart_path()?;
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    } else {
        std::fs::create_dir_all(path.parent().ok_or("No autostart directory")?)
            .map_err(|e| e.to_string())?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        state::atomic_write(&path, state::autostart_entry(&executable)?.as_bytes())
    }
}

fn setting_text(title: &str, key: &str, cfg: &Config) -> String {
    let state = match key {
        "theme" => return format!("Theme: {} (click to cycle)", cfg.theme),
        "autostart" => {
            let on = autostart_path().is_ok_and(|p| p.exists());
            return format!("Start at login: {}", if on { "on" } else { "off" });
        }
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

pub fn diagnostics() {
    let usage_source = system::backend()
        .map(|(b, p)| format!("{} ({})", b.label(), p.display()))
        .unwrap_or_else(|| "unavailable".into());
    let bus = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>).is_ok();
    let sqlite = if system::sqlite3_available() {
        "found"
    } else {
        "missing (KDE usage import disabled)"
    };
    println!(
        "Luma {}\nDesktop: {}\nSession: {}\nGTK: {}.{}\nRust: {}\nConfiguration: {}\nShortcut backend: portal registration on explicit setup; desktop-managed fallback\nSession bus: {}\nUsage source: {}\nSQLite CLI (KDE usage): {}\nInstalled applications: {}",
        env!("CARGO_PKG_VERSION"),
        std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
        std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
        gtk::major_version(),
        gtk::minor_version(),
        env!("LUMA_RUSTC"),
        state::directory("config").display(),
        if bus { "accessible" } else { "unavailable" },
        usage_source,
        sqlite,
        discover().len()
    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use gdk::Key as K;

    #[test]
    fn settings_accepts_comma_and_plus_only_with_control() {
        for key in [K::comma, K::plus, K::equal, K::KP_Add] {
            assert_eq!(shortcut(key, true), Some(Shortcut::Settings), "{key:?}");
            assert_eq!(shortcut(key, false), None, "plain {key:?}");
        }
        assert_eq!(shortcut(K::a, true), None);
    }

    #[test]
    fn letter_shortcuts_ignore_caps_lock() {
        for (key, expected) in [
            (K::d, Shortcut::Favorite),
            (K::D, Shortcut::Favorite),
            (K::J, Shortcut::Next),
            (K::n, Shortcut::Next),
            (K::K, Shortcut::Prev),
            (K::P, Shortcut::Prev),
            (K::_1, Shortcut::Jump(0)),
            (K::_9, Shortcut::Jump(8)),
            (K::Return, Shortcut::Actions),
            (K::Delete, Shortcut::Forget),
        ] {
            assert_eq!(shortcut(key, true), Some(expected), "{key:?}");
            assert_eq!(shortcut(key, false), None, "plain {key:?}");
        }
        assert_eq!(shortcut(K::_0, true), None);
    }

    #[test]
    fn navigation_keys_work_without_control() {
        assert_eq!(shortcut(K::Escape, false), Some(Shortcut::Escape));
        assert_eq!(shortcut(K::Down, false), Some(Shortcut::Next));
        assert_eq!(shortcut(K::Up, false), Some(Shortcut::Prev));
    }

    /// All widget checks share one test: gtk-rs binds GTK to the thread that
    /// initialises it, so a second test thread could not use it. Skipped when
    /// no display is reachable (run under Xvfb or Broadway in CI).
    #[test]
    fn gtk_session() {
        if gtk::init().is_err() {
            eprintln!("skipped: no display available");
            return;
        }
        crate::theme::assert_stylesheets_parse();
        focus_dismissal_and_settings_save_rollback();
    }

    fn checkbox(widget: &gtk::Widget) -> Option<gtk::CheckButton> {
        if let Some(check) = widget.downcast_ref::<gtk::CheckButton>() {
            return Some(check.clone());
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            if let Some(found) = checkbox(&current) {
                return Some(found);
            }
            child = current.next_sibling();
        }
        None
    }

    fn focus_dismissal_and_settings_save_rollback() {
        let root = std::env::temp_dir().join(format!("luma-focus-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &root);
        std::env::set_var("XDG_STATE_HOME", &root);
        let app = gtk::Application::builder()
            .application_id("io.github.luma_launcher.FocusTest")
            .flags(gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gio::Cancellable>).unwrap();
        let launcher = Launcher::new(&app);
        let window = launcher.0.window.clone();
        launcher.command("--show");
        assert!(window.is_visible());

        // No compositor events have been dispatched, so the window is inactive.
        assert!(!window.is_active());
        window.notify("is-active");
        assert!(!window.is_visible());
        assert!(!launcher.0.visible.get());

        launcher.command("--toggle");
        assert!(window.is_visible());
        assert!(launcher.0.visible.get());
        launcher.hide();

        launcher.command("--settings");
        let settings = launcher.0.settings.borrow().clone().unwrap();
        let control = checkbox(settings.upcast_ref()).expect("native settings checkbox");
        assert!(!control.is_active());
        let directory = root.join("luma-launcher");
        std::fs::write(&directory, "blocks configuration writes").unwrap();
        control.set_active(true);
        assert!(!control.is_active());
        assert!(!launcher.0.config.borrow().compact);

        std::fs::remove_file(&directory).unwrap();
        control.set_active(true);
        assert!(control.is_active());
        assert!(launcher.0.config.borrow().compact);
        assert!(
            state::load::<Config>(&directory.join("config.toml"))
                .unwrap()
                .compact
        );
        settings.destroy();
        window.destroy();
        std::fs::remove_dir_all(root).unwrap();
    }
}
