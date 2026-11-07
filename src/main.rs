#![forbid(unsafe_code)]
mod portal;
mod theme;
mod ui;
use gtk4::{self as gtk, prelude::*};
fn main() -> glib::ExitCode {
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
            return glib::ExitCode::SUCCESS;
        }
        Some("--diagnostics") => {
            ui::diagnostics();
            return glib::ExitCode::SUCCESS;
        }
        _ => {}
    }
    if let Err(e) = gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>) {
        eprintln!("Cannot access session D-Bus; refusing to start an uncoordinated instance: {e}");
        std::process::exit(1);
    }
    let app = gtk::Application::builder()
        .application_id("io.github.luma_launcher.LumaLauncher")
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
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
    app.run()
}
