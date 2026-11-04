use gio::prelude::*;
use std::{
    fs::{self, File, OpenOptions},
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
    app: &gio::DesktopAppInfo,
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
    let mut stderr = stdout.try_clone().map_err(|e| e.to_string())?;
    let mut stdin = File::open("/dev/null").map_err(|e| e.to_string())?;
    app.launch_uris_as_manager_with_fds(
        &[],
        Some(context),
        glib::SpawnFlags::SEARCH_PATH,
        None,
        None,
        &mut stdin,
        &mut stdout,
        &mut stderr,
    )
    .map_err(|error| format!("Could not open {}: {error}", app.display_name()))
}
