use gio::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::Path, process::Command};

#[test]
fn installed_entry_uses_its_binary_without_session_path() {
    let root = std::env::temp_dir().join(format!("luma-installer-{}", std::process::id()));
    let project = root.join("project");
    for path in ["scripts", "packaging", "assets", "target/release"] {
        fs::create_dir_all(project.join(path)).unwrap();
    }
    for file in [
        "scripts/install.sh",
        "packaging/io.github.luma_launcher.LumaLauncher.desktop",
        "assets/io.github.luma_launcher.LumaLauncher.svg",
    ] {
        fs::copy(file, project.join(file)).unwrap();
    }
    if Path::new("scripts/desktop-exec.sh").exists() {
        fs::copy(
            "scripts/desktop-exec.sh",
            project.join("scripts/desktop-exec.sh"),
        )
        .unwrap();
    }
    let stub = project.join("target/release/luma-launcher");
    fs::write(
        &stub,
        "#!/bin/sh\nprintf launched > \"$LUMA_TEST_MARKER\"\n",
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    let bin = root.join("bin with spaces \"quoted\" \\ slash");
    let data = root.join("data");
    assert!(Command::new("bash")
        .arg(project.join("scripts/install.sh"))
        .env("LUMA_BIN_DIR", &bin)
        .env("XDG_DATA_HOME", &data)
        .status()
        .unwrap()
        .success());
    let entry = data.join("applications/io.github.luma_launcher.LumaLauncher.desktop");
    let info = gio::DesktopAppInfo::from_filename(&entry).unwrap();
    assert!(
        info.commandline()
            .unwrap()
            .to_string_lossy()
            .contains("bin with spaces"),
        "desktop entry must reference its installed executable"
    );
    let marker = root.join("launched");
    let context = gio::AppLaunchContext::new();
    context.setenv("LUMA_TEST_MARKER", marker.to_str().unwrap());
    context.setenv("PATH", "/usr/bin:/bin");
    info.launch(&[], Some(&context)).unwrap();
    for _ in 0..100 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(&marker).unwrap(), "launched");
    fs::remove_file(&marker).unwrap();
    let keyfile = glib::KeyFile::new();
    keyfile
        .load_from_data(
            &luma_launcher::state::autostart_entry(&bin.join("luma-launcher")).unwrap(),
            glib::KeyFileFlags::NONE,
        )
        .unwrap();
    gio::DesktopAppInfo::from_keyfile(&keyfile)
        .unwrap()
        .launch(&[], Some(&context))
        .unwrap();
    for _ in 0..100 {
        if marker.exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(fs::read_to_string(&marker).unwrap(), "launched");
    fs::remove_dir_all(root).unwrap();
}
