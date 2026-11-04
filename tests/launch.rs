use gio::prelude::*;
use luma_launcher::launch;
use std::{
    fs,
    io::Write,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

fn wait_for(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !path.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        path.exists(),
        "launched application did not create {path:?}"
    );
}

#[test]
fn application_output_is_logged_without_polluting_launcher_output() {
    let role = std::env::var("LUMA_LAUNCH_TEST_ROLE").unwrap_or_default();
    if role == "app" {
        let root = std::env::var_os("LUMA_LAUNCH_TEST_ROOT").unwrap();
        assert_eq!(std::env::current_dir().unwrap(), Path::new(&root));
        for (key, expected) in [
            ("DISPLAY", ":77"),
            ("WAYLAND_DISPLAY", "wayland-test"),
            ("XAUTHORITY", "/test/session/Xauthority"),
        ] {
            assert_eq!(std::env::var(key).unwrap(), expected);
        }
        println!("CHILD_APP_STDOUT");
        eprintln!("CHILD_APP_STDERR");
        std::io::stdout().flush().unwrap();
        fs::write(Path::new(&root).join("completed"), "ok").unwrap();
        return;
    }
    if role == "launcher" {
        let root = std::env::var_os("LUMA_LAUNCH_TEST_ROOT").unwrap();
        let log_path = std::env::var_os("LUMA_LAUNCH_TEST_LOG").unwrap();
        let key = glib::KeyFile::new();
        key.set_string("Desktop Entry", "Type", "Application");
        key.set_string("Desktop Entry", "Name", "Launch test");
        key.set_string("Desktop Entry", "Path", Path::new(&root).to_str().unwrap());
        key.set_string(
            "Desktop Entry",
            "Exec",
            &format!(
                "\"{}\" --exact application_output_is_logged_without_polluting_launcher_output --nocapture",
                std::env::current_exe().unwrap().display()
            ),
        );
        let app = gio::DesktopAppInfo::from_keyfile(&key).unwrap();
        let context = gio::AppLaunchContext::new();
        context.setenv("LUMA_LAUNCH_TEST_ROLE", "app");
        context.setenv("DISPLAY", ":77");
        context.setenv("WAYLAND_DISPLAY", "wayland-test");
        context.setenv("XAUTHORITY", "/test/session/Xauthority");
        launch::desktop(&app, &context, Path::new(&log_path)).unwrap();
        wait_for(&Path::new(&root).join("completed"));
        return;
    }

    let root = std::env::temp_dir().join(format!("luma-launch-output-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    for scenario in ["normal", "unwritable", "rotation"] {
        let work = root.join(format!("{scenario} working directory"));
        fs::create_dir(&work).unwrap();
        let log = work.join("logs/applications.log");
        if scenario == "unwritable" {
            fs::write(work.join("logs"), "not a directory").unwrap();
        } else if scenario == "rotation" {
            fs::create_dir_all(log.parent().unwrap()).unwrap();
            fs::write(&log, vec![b'x'; 5 * 1024 * 1024]).unwrap();
        }
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "application_output_is_logged_without_polluting_launcher_output",
                "--nocapture",
            ])
            .env("LUMA_LAUNCH_TEST_ROLE", "launcher")
            .env("LUMA_LAUNCH_TEST_ROOT", &work)
            .env("LUMA_LAUNCH_TEST_LOG", &log)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        for stream in [&output.stdout, &output.stderr] {
            assert!(
                !String::from_utf8_lossy(stream).contains("CHILD_APP_"),
                "child output leaked into Luma's terminal: {output:?}"
            );
        }
        assert_eq!(fs::read_to_string(work.join("completed")).unwrap(), "ok");
        if scenario == "normal" || scenario == "rotation" {
            let text = fs::read_to_string(&log).unwrap();
            assert!(text.contains("CHILD_APP_STDOUT"));
            assert!(text.contains("CHILD_APP_STDERR"));
            assert_eq!(
                fs::metadata(&log).unwrap().permissions().mode() & 0o777,
                0o600
            );
            if scenario == "rotation" {
                assert_eq!(
                    fs::metadata(log.with_extension("log.1")).unwrap().len(),
                    5 * 1024 * 1024
                );
            }
        }
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn removed_executable_returns_a_launch_error() {
    let root = std::env::temp_dir().join(format!("luma-launch-removed-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let executable = root.join("removed");
    symlink("/usr/bin/true", &executable).unwrap();
    let key = glib::KeyFile::new();
    key.set_string("Desktop Entry", "Type", "Application");
    key.set_string("Desktop Entry", "Name", "Removed executable");
    key.set_string("Desktop Entry", "Exec", executable.to_str().unwrap());
    let app = gio::DesktopAppInfo::from_keyfile(&key).unwrap();
    fs::remove_file(&executable).unwrap();
    let error = launch::desktop(
        &app,
        &gio::AppLaunchContext::new(),
        &root.join("applications.log"),
    )
    .unwrap_err();
    assert!(error.contains("removed"), "{error}");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn quoted_arguments_are_not_interpreted_as_shell_commands() {
    let root = std::env::temp_dir().join(format!("luma-launch-arguments-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let key = glib::KeyFile::new();
    key.set_string("Desktop Entry", "Type", "Application");
    key.set_string("Desktop Entry", "Name", "Quoted arguments");
    key.set_string("Desktop Entry", "Path", root.to_str().unwrap());
    key.set_string(
        "Desktop Entry",
        "Exec",
        "/usr/bin/touch \"literal ; $(touch injected)\"",
    );
    let app = gio::DesktopAppInfo::from_keyfile(&key).unwrap();
    launch::desktop(
        &app,
        &gio::AppLaunchContext::new(),
        &root.join("applications.log"),
    )
    .unwrap();
    wait_for(&root.join("literal ; $(touch injected)"));
    assert!(!root.join("injected").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn desktop_actions_execute_the_declared_command() {
    let root = std::env::temp_dir().join(format!("luma-launch-action-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let key = glib::KeyFile::new();
    key.set_string("Desktop Entry", "Type", "Application");
    key.set_string("Desktop Entry", "Name", "Desktop action");
    key.set_string("Desktop Entry", "Exec", "/usr/bin/true");
    key.set_string("Desktop Entry", "Path", root.to_str().unwrap());
    key.set_string("Desktop Entry", "Actions", "Open;");
    key.set_string("Desktop Action Open", "Name", "Open test");
    key.set_string(
        "Desktop Action Open",
        "Exec",
        "/usr/bin/touch \"action completed\"",
    );
    let app = gio::DesktopAppInfo::from_keyfile(&key).unwrap();
    assert_eq!(app.list_actions(), ["Open"]);
    app.launch_action("Open", Some(&gio::AppLaunchContext::new()));
    wait_for(&root.join("action completed"));
    fs::remove_dir_all(root).unwrap();
}
