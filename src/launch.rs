use gio_unix::prelude::*;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

const LOG_LIMIT: u64 = 5 * 1024 * 1024;

fn prepare_log(path: &Path) -> std::io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if fs::metadata(path).is_ok_and(|metadata| metadata.len() >= LOG_LIMIT) {
        let previous = path.with_extension("log.1");
        if previous.exists() {
            fs::remove_file(&previous)?;
        }
        fs::rename(path, previous)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(file)
}

pub fn desktop(
    app: &gio_unix::DesktopAppInfo,
    context: &gio::AppLaunchContext,
    log_path: &Path,
) -> Result<(), String> {
    let label = app.id().map(|id| id.to_string()).unwrap_or_default();
    run(app, &label, context, log_path)
}

fn run(
    app: &gio_unix::DesktopAppInfo,
    label: &str,
    context: &gio::AppLaunchContext,
    log_path: &Path,
) -> Result<(), String> {
    let log = prepare_log(log_path);
    let mut stdout = match log {
        Ok(file) => file,
        Err(error) => {
            eprintln!(
                "level=warning message=\"Could not write application log\" path={log_path:?} error={error:?}"
            );
            OpenOptions::new()
                .write(true)
                .open("/dev/null")
                .map_err(|e| e.to_string())?
        }
    };
    // One line per launch so interleaved output stays attributable.
    let _ = writeln!(
        stdout,
        "--- unix={} launch {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        label
    );
    let stderr = stdout.try_clone().map_err(|e| e.to_string())?;
    let stdin = File::open("/dev/null").map_err(|e| e.to_string())?;
    app.launch_uris_as_manager_with_fds(
        &[],
        Some(context),
        glib::SpawnFlags::SEARCH_PATH,
        None,
        None,
        Some(&stdin),
        Some(&stdout),
        Some(&stderr),
    )
    .map_err(|error| format!("Could not open {}: {error}", app.display_name()))
}

/// Launch a declared desktop action with the same output redirection as
/// `desktop`: the action's `Exec` replaces the entry's own and GIO does the
/// rest. Entries without a readable file fall back to plain GIO.
pub fn action(
    app: &gio_unix::DesktopAppInfo,
    action: &str,
    context: &gio::AppLaunchContext,
    log_path: &Path,
) -> Result<(), String> {
    let launch_plain = || {
        app.launch_action(action, Some(context));
        Ok(())
    };
    let Some(file) = app.filename() else {
        return launch_plain();
    };
    let key = glib::KeyFile::new();
    let group = format!("Desktop Action {action}");
    let Some(exec) = key
        .load_from_file(&file, glib::KeyFileFlags::NONE)
        .ok()
        .and_then(|()| key.string(&group, "Exec").ok())
    else {
        return launch_plain();
    };
    key.set_string("Desktop Entry", "Exec", &exec);
    let _ = key.remove_key("Desktop Entry", "DBusActivatable");
    let label = format!(
        "{} action {action}",
        app.id().map(|id| id.to_string()).unwrap_or_default()
    );
    match gio_unix::DesktopAppInfo::from_keyfile(&key) {
        Some(entry) => run(&entry, &label, context, log_path),
        None => launch_plain(),
    }
}
