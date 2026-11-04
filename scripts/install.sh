#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
bin_dir=${LUMA_BIN_DIR:-"$HOME/.local/bin"}
data_dir=${XDG_DATA_HOME:-"$HOME/.local/share"}
[[ $data_dir == /* ]] || data_dir="$HOME/.local/share"
if [[ $bin_dir != /* || $# != 0 ]]; then
	printf 'Use an absolute LUMA_BIN_DIR; install.sh takes no arguments.\n' >&2
	exit 2
fi
source scripts/desktop-exec.sh
exec_path=$(desktop_exec "$bin_dir/luma-launcher")
test -x target/release/luma-launcher
install -Dm755 target/release/luma-launcher "$bin_dir/luma-launcher"
mkdir -p -- "$data_dir/applications"
entry=$(mktemp "$data_dir/applications/.luma-XXXXXX")
trap 'rm -f -- "$entry"' EXIT
while IFS= read -r line; do
	case $line in
	Exec=luma-launcher\ *) printf 'Exec=%s %s\n' "$exec_path" "${line#Exec=luma-launcher }" ;;
	*) printf '%s\n' "$line" ;;
	esac
done <packaging/io.github.luma_launcher.LumaLauncher.desktop >"$entry"
chmod 644 "$entry"
mv -f -- "$entry" "$data_dir/applications/io.github.luma_launcher.LumaLauncher.desktop"
install -Dm644 assets/io.github.luma_launcher.LumaLauncher.svg "$data_dir/icons/hicolor/scalable/apps/io.github.luma_launcher.LumaLauncher.svg"
if command -v update-desktop-database >/dev/null; then
	update-desktop-database "$data_dir/applications"
fi
printf 'Installed. Ensure %s is in PATH. Configure Alt+Space to run luma-launcher --toggle.\n' "$bin_dir"
