#!/usr/bin/env bash
set -euo pipefail
config_dir=${XDG_CONFIG_HOME:-"$HOME/.config"}
[[ $config_dir == /* ]] || config_dir="$HOME/.config"
if [[ $# != 1 ]]; then
	printf 'Usage: %s enable|disable\n' "$0" >&2
	exit 2
fi
# shellcheck source=scripts/desktop-exec.sh
source "$(dirname -- "$0")/desktop-exec.sh"
file="$config_dir/autostart/io.github.luma_launcher.LumaLauncher.desktop"
case ${1:-} in
enable)
	executable=$(command -v luma-launcher)
	executable=$(realpath -e -- "$executable")
	exec_path=$(desktop_exec "$executable")
	mkdir -p -- "$config_dir/autostart"
	entry=$(mktemp "$config_dir/autostart/.luma-XXXXXX")
	trap 'rm -f -- "$entry"' EXIT
	printf '[Desktop Entry]\nType=Application\nName=Luma Launcher\nExec=%s --background\nTerminal=false\n' "$exec_path" >"$entry"
	mv -f -- "$entry" "$file"
	;;
disable) rm -f -- "$file" ;;
*)
	printf 'Usage: %s enable|disable\n' "$0" >&2
	exit 2
	;;
esac
