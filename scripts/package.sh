#!/usr/bin/env bash
# Build dist/luma-launcher_<version>_<arch>.deb from target/release.
# Static metadata (maintainer, homepage, recommends, description) lives in
# debian/control; the version comes from Cargo.toml and must match the top
# entry of debian/changelog.
set -euo pipefail
cd -- "$(dirname -- "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
changelog_version=$(sed -n '1s/^luma-launcher (\(.*\)) .*/\1/p' debian/changelog)
if [[ -z $version || $version != "$changelog_version" ]]; then
	printf 'Cargo.toml version %q does not match the top debian/changelog entry %q.\n' "$version" "$changelog_version" >&2
	exit 1
fi
changelog_date=$(sed -n '/^ -- /{s/^ -- .*>  //;p;q}' debian/changelog)
# Fixed timestamp so rebuilding the same version gives the same package contents.
man_date=$(date -u -d "$changelog_date" '+%B %Y')

source_field() { sed -n "s/^$1: //p" debian/control | head -n 1; }

stage=$(mktemp -d)
trap 'rm -rf -- "$stage"' EXIT
chmod 755 "$stage"
doc="$stage/usr/share/doc/luma-launcher"
install -Dm755 target/release/luma-launcher "$stage/usr/bin/luma-launcher"
install -Dm644 packaging/io.github.luma_launcher.LumaLauncher.desktop "$stage/usr/share/applications/io.github.luma_launcher.LumaLauncher.desktop"
install -Dm644 assets/io.github.luma_launcher.LumaLauncher.svg "$stage/usr/share/icons/hicolor/scalable/apps/io.github.luma_launcher.LumaLauncher.svg"
install -Dm644 debian/copyright "$doc/copyright"
gzip -9n -c README.md >"$doc/README.md.gz"
chmod 644 "$doc/README.md.gz"
# Native package (no Debian revision): the changelog is installed as changelog.gz.
gzip -9n -c debian/changelog >"$doc/changelog.gz"
chmod 644 "$doc/changelog.gz"
mkdir -p "$stage/usr/share/man/man1"
sed -e "s/@VERSION@/$version/" -e "s/@DATE@/$man_date/" packaging/luma-launcher.1 |
	gzip -9n >"$stage/usr/share/man/man1/luma-launcher.1.gz"
chmod 644 "$stage/usr/share/man/man1/luma-launcher.1.gz"

mkdir -p "$stage/DEBIAN" dist
# Packaged directories are 0755 whatever the build user's umask is.
find "$stage" -type d -exec chmod 755 {} +
deps=$(dpkg-shlibdeps -O -e"$stage/usr/bin/luma-launcher")
deps=${deps#shlibs:Depends=}
test -n "$deps"
arch=$(dpkg --print-architecture)
{
	printf 'Package: luma-launcher\nVersion: %s\nArchitecture: %s\n' "$version" "$arch"
	for field in Maintainer Homepage Section Priority; do
		printf '%s: %s\n' "$field" "$(source_field "$field")"
	done
	printf 'Installed-Size: %s\n' "$(du -sk --exclude=DEBIAN "$stage" | cut -f1)"
	printf 'Depends: %s\n' "$deps"
	# Remaining binary-stanza fields (Recommends, Suggests, Description).
	sed -n '/^Package:/,$p' debian/control | sed 1d | grep -Ev '^(Architecture|Depends):'
} >"$stage/DEBIAN/control"
# Checksums let `debsums` verify an installed package.
(cd "$stage" && find usr -type f -exec md5sum {} + >DEBIAN/md5sums)
chmod 644 "$stage/DEBIAN/md5sums"
chmod 755 "$stage/DEBIAN"
dpkg-deb --root-owner-group --build "$stage" "dist/luma-launcher_${version}_${arch}.deb"
