# Installation and removal

Build with `cargo build --release`. For normal development install Debian's
`build-essential pkg-config libglib2.0-dev libgtk-4-dev` packages. Root is never
needed to run Luma or install it locally.

`bash scripts/install.sh` copies the binary to `~/.local/bin` (override with
`LUMA_BIN_DIR`), the desktop entry to `$XDG_DATA_HOME/applications`, and its SVG
to `$XDG_DATA_HOME/icons/hicolor/scalable/apps`. Data home defaults to
`~/.local/share`. Installed application desktop entries use an absolute executable path. Add the binary directory to PATH for terminal commands. Executable paths with percent, equals, or control characters are rejected before installation.

Autostart is off initially. Use Settings or `scripts/autostart.sh enable|disable`.
Only Luma's own entry under `$XDG_CONFIG_HOME/autostart` is managed. Autostart uses an absolute executable path.

Configuration: `$XDG_CONFIG_HOME/luma-launcher/config.toml`, defaulting to
`~/.config/luma-launcher/config.toml`. Launch state:
`$XDG_STATE_HOME/luma-launcher/history.toml`, defaulting to
`~/.local/state/luma-launcher/history.toml`. Settings writes config; width,
font_size, accent (#RRGGBB), results, and shortcut can also be edited in TOML.
Restart after external edits. Width is limited to 360–1200, font to 12–32, results
to 1–20. Copy and repair corrupt files, then restart; Luma preserves the originals.

`bash scripts/package.sh` builds an amd64 package on this host using
dpkg-shlibdeps-generated dependencies. This package targets Testing/Unstable;
do not assume it installs on Stable. Build with `packaging/Containerfile` in an
accessible container environment for Debian 13 libraries, extract `/build/dist`,
then test installation in a fresh Debian Stable VM. That container path is untested.

Stop Luma with `--quit` before uninstalling. `scripts/uninstall.sh` preserves
preferences; `--purge` removes only Luma's configuration and state directories.
Remove user-created keyboard bindings manually. A Debian installation is removed
through the package manager rather than the local uninstall script.
