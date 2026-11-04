use std::process::Command;

#[test]
fn informational_flags_do_not_bypass_argument_validation() {
    for args in [["--version", "--hide"], ["--diagnostics", "--unknown"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_luma-launcher"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
    }
}

#[test]
fn gio_forwards_unregistered_command_flags() {
    use gio::prelude::*;
    let app = gio::Application::new(
        None,
        gio::ApplicationFlags::NON_UNIQUE | gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );
    app.connect_command_line(|_, cmd| {
        assert_eq!(cmd.arguments()[1], "--show");
        0
    });
    assert_eq!(
        app.run_with_args(&["luma-test", "--show"]),
        glib::ExitCode::SUCCESS
    );
}
