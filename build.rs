fn main() {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    if let Ok(output) = std::process::Command::new(rustc).arg("--version").output() {
        println!(
            "cargo:rustc-env=LUMA_RUSTC={}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }
}
