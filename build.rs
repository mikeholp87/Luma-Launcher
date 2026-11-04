fn main() {
    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    if let Ok(output) = std::process::Command::new(rustc).arg("--version").output() {
        println!(
            "cargo:rustc-env=LUMA_RUSTC={}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }
    // Runtime-only development environments lack the unversioned linker symlink.
    println!("cargo:rustc-link-arg=-l:libgtk-4.so.1");
    println!("cargo:rustc-link-arg=-l:libgobject-2.0.so.0");
}
