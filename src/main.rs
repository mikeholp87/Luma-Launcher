mod gtk;
mod portal;
mod theme;
mod ui;
use gio::prelude::*;
use glib::translate::*;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let known = [
        "--show",
        "--hide",
        "--toggle",
        "--settings",
        "--refresh",
        "--background",
        "--quit",
        "--version",
        "--diagnostics",
    ];
    if args.len() > 2 || args.iter().skip(1).any(|a| !known.contains(&a.as_str())) {
        eprintln!("Usage: luma-launcher [--show|--hide|--toggle|--settings|--refresh|--background|--quit|--diagnostics|--version]");
        std::process::exit(2);
    }
    match args.get(1).map(String::as_str) {
        Some("--version") => {
            println!("Luma Launcher {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        Some("--diagnostics") => {
            ui::diagnostics();
            return;
        }
        _ => {}
    }
    if let Err(e) = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>) {
        eprintln!("Cannot access session D-Bus; refusing to start an uncoordinated instance: {e}");
        std::process::exit(1);
    }
    // SAFETY: GTK creates a GApplication subclass; ownership is transferred once.
    let app: gio::Application = unsafe {
        from_glib_full(
            gtk::gtk_application_new(
                gtk::c("io.github.luma_launcher.LumaLauncher").as_ptr(),
                gio::ApplicationFlags::HANDLES_COMMAND_LINE.into_glib(),
            )
            .cast::<gio::ffi::GApplication>(),
        )
    };
    let runtime = std::rc::Rc::new(std::cell::RefCell::new(None));
    let r = runtime.clone();
    app.connect_startup(move |app| {
        *r.borrow_mut() = Some(ui::Launcher::new(app));
    });
    app.connect_command_line(move |_, cmd| {
        let args = cmd.arguments();
        let arg = args.get(1).and_then(|s| s.to_str()).unwrap_or("--show");
        if let Some(l) = runtime.borrow().as_ref() {
            l.command(arg)
        };
        0.into()
    });
    app.run();
}
