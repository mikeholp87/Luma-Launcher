#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
test -x target/release/luma-launcher
test_root=$(mktemp -d)
trap 'rm -rf -- "$test_root"' EXIT
export LUMA_BIN_DIR="$test_root/bin with spaces"
export XDG_DATA_HOME="$test_root/data"
export XDG_CONFIG_HOME="$test_root/config"
export XDG_STATE_HOME="$test_root/state"
export PATH="$LUMA_BIN_DIR:$PATH"
bash scripts/install.sh
cmp target/release/luma-launcher "$LUMA_BIN_DIR/luma-launcher"
"$LUMA_BIN_DIR/luma-launcher" --version
desktop-file-validate "$XDG_DATA_HOME/applications/io.github.luma_launcher.LumaLauncher.desktop"
test -f "$XDG_DATA_HOME/icons/hicolor/scalable/apps/io.github.luma_launcher.LumaLauncher.svg"
bash scripts/autostart.sh enable
desktop-file-validate "$XDG_CONFIG_HOME/autostart/io.github.luma_launcher.LumaLauncher.desktop"
bash scripts/autostart.sh disable
test ! -e "$XDG_CONFIG_HOME/autostart/io.github.luma_launcher.LumaLauncher.desktop"
mkdir -p "$XDG_CONFIG_HOME/luma-launcher" "$XDG_STATE_HOME/luma-launcher"
printf 'theme = "dark"\n' >"$XDG_CONFIG_HOME/luma-launcher/config.toml"
printf '[favorites]\n' >"$XDG_STATE_HOME/luma-launcher/history.toml"
printf 'unrelated file\n' >"$XDG_CONFIG_HOME/keep.txt"
if bash scripts/uninstall.sh --purge unexpected; then
	printf 'FAIL: invalid uninstall arguments accepted\n' >&2
	exit 1
fi
test -f "$XDG_CONFIG_HOME/luma-launcher/config.toml"
if LUMA_BIN_DIR="$test_root/bin%invalid" bash scripts/install.sh; then
	printf 'FAIL: unsupported executable path accepted\n' >&2
	exit 1
fi
test ! -e "$test_root/bin%invalid"
bash scripts/uninstall.sh
test ! -e "$LUMA_BIN_DIR/luma-launcher"
test ! -e "$XDG_DATA_HOME/applications/io.github.luma_launcher.LumaLauncher.desktop"
test -f "$XDG_CONFIG_HOME/luma-launcher/config.toml"
test -f "$XDG_STATE_HOME/luma-launcher/history.toml"
bash scripts/uninstall.sh --purge
test ! -e "$XDG_CONFIG_HOME/luma-launcher"
test ! -e "$XDG_STATE_HOME/luma-launcher"
test -f "$XDG_CONFIG_HOME/keep.txt"
printf 'PASS: local install, spaces in paths, desktop entry, icons, autostart, preservation, purge\n'
