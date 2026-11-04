#!/usr/bin/env bash
set -euo pipefail
if [[ $# -gt 1 || ($# == 1 && $1 != --purge) ]]; then
	printf 'Usage: %s [--purge]\n' "$0" >&2
	exit 2
fi
bin_dir=${LUMA_BIN_DIR:-"$HOME/.local/bin"}
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
config_dir=${XDG_CONFIG_HOME:-"$HOME/.config"}
state_dir=${XDG_STATE_HOME:-"$HOME/.local/state"}
[[ $data_dir == /* ]] || data_dir="$HOME/.local/share"
[[ $config_dir == /* ]] || config_dir="$HOME/.config"
[[ $state_dir == /* ]] || state_dir="$HOME/.local/state"
if [[ $bin_dir != /* ]]; then
	printf 'LUMA_BIN_DIR must be absolute.\n' >&2
	exit 2
fi
rm -f -- "$bin_dir/luma-launcher" "$data_dir/applications/io.github.luma_launcher.LumaLauncher.desktop" "$data_dir/icons/hicolor/scalable/apps/io.github.luma_launcher.LumaLauncher.svg" "$config_dir/autostart/io.github.luma_launcher.LumaLauncher.desktop"
if [[ ${1:-} == --purge ]]; then
	rm -rf -- "$config_dir/luma-launcher" "$state_dir/luma-launcher"
fi
printf 'Removed Luma files. Preferences preserved unless --purge was supplied. Remove any desktop shortcut you created.\n'
