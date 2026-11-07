#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
install -Dm755 target/release/luma-launcher "$stage/usr/bin/luma-launcher"
install -Dm644 packaging/io.github.luma_launcher.LumaLauncher.desktop "$stage/usr/share/applications/io.github.luma_launcher.LumaLauncher.desktop"
install -Dm644 assets/io.github.luma_launcher.LumaLauncher.svg "$stage/usr/share/icons/hicolor/scalable/apps/io.github.luma_launcher.LumaLauncher.svg"
install -Dm644 LICENSE "$stage/usr/share/doc/luma-launcher/copyright"
install -Dm644 README.md "$stage/usr/share/doc/luma-launcher/README.md"
mkdir -p "$stage/DEBIAN" dist
deps=$(dpkg-shlibdeps -O -e"$stage/usr/bin/luma-launcher")
deps=${deps#shlibs:Depends=}
test -n "$deps"
arch=$(dpkg --print-architecture)
cat >"$stage/DEBIAN/control" <<CONTROL
Package: luma-launcher
Version: 0.1.0
Architecture: $arch
Maintainer: Luma Launcher contributors <noreply@example.invalid>
Depends: $deps
Section: utils
Priority: optional
Description: Native GTK4 application launcher
 Keyboard-driven application search with local favorites and history.
CONTROL
dpkg-deb --root-owner-group --build "$stage" "dist/luma-launcher_0.1.0_${arch}.deb"
