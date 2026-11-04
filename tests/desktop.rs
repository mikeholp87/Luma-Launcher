use gio::prelude::*;

fn desktop(extra: &str) -> gio::DesktopAppInfo {
    let key = glib::KeyFile::new();
    key.load_from_data(
        &format!("[Desktop Entry]\nType=Application\nName=Luma Test\nExec=/usr/bin/true\nIcon=application-x-executable\n{extra}"),
        glib::KeyFileFlags::NONE,
    ).unwrap();
    gio::DesktopAppInfo::from_keyfile(&key).unwrap()
}
#[test]
fn visibility_and_metadata() {
    let visible = desktop("Comment=Fixture application\n");
    assert!(visible.should_show());
    assert_eq!(visible.name(), "Luma Test");
    assert!(visible.icon().is_some());
    assert!(!desktop("NoDisplay=true\n").should_show());
    assert!(desktop("Hidden=true\n").is_hidden());
}
#[test]
fn launches_without_shell_interpolation() {
    desktop("")
        .launch(&[], None::<&gio::AppLaunchContext>)
        .unwrap();
}
#[test]
fn malformed_desktop_entry_rejected() {
    let key = glib::KeyFile::new();
    assert!(key
        .load_from_data("not a desktop entry", glib::KeyFileFlags::NONE)
        .is_err());
}

#[test]
fn xdg_precedence_and_duplicate_ids() {
    let root = std::env::temp_dir().join(format!("luma-xdg-{}", std::process::id()));
    let user = root.join("user");
    let system = root.join("system");
    std::fs::create_dir_all(user.join("applications")).unwrap();
    std::fs::create_dir_all(system.join("applications")).unwrap();
    let valid = "[Desktop Entry]\nType=Application\nName=Fixture\nExec=/usr/bin/true\n";
    for dir in [&user, &system] {
        std::fs::write(dir.join("applications/duplicate.desktop"), valid).unwrap();
    }
    std::fs::write(system.join("applications/masked.desktop"), valid).unwrap();
    std::fs::write(
        user.join("applications/masked.desktop"),
        format!("{valid}Hidden=true\n"),
    )
    .unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_luma-launcher"))
        .arg("--diagnostics")
        .env("XDG_DATA_HOME", &user)
        .env("XDG_DATA_DIRS", &system)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("Installed applications: 1"),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
    std::fs::remove_dir_all(root).unwrap();
}
