#!/usr/bin/env bash
# Turn a directory of .deb files into a signed *flat* apt repository.
#
#   GNUPGHOME=... bash scripts/build-apt-repo.sh <dir> <public-key.asc>
#
# The signing key is the first secret key in GNUPGHOME. After signing, the result
# is checked against <public-key.asc>, so publishing fails if the secret and the
# public key users trust ever disagree.
set -euo pipefail

dir=${1:?usage: build-apt-repo.sh <dir> <public-key.asc>}
pubkey=$(realpath "${2:?usage: build-apt-repo.sh <dir> <public-key.asc>}")
here=$(dirname -- "$(realpath "$0")")

cd -- "$dir"
shopt -s nullglob
debs=(*.deb)
if ((${#debs[@]} == 0)); then
	echo "No .deb files in $dir" >&2
	exit 1
fi

dpkg-scanpackages --multiversion . >Packages
gzip -9nk Packages
apt-ftparchive \
	-o APT::FTPArchive::Release::Origin="Luma Launcher" \
	-o APT::FTPArchive::Release::Label="Luma Launcher" \
	-o APT::FTPArchive::Release::Suite=stable \
	-o APT::FTPArchive::Release::Architectures=amd64 \
	-o APT::FTPArchive::Release::Description="Luma Launcher apt repository" \
	release . >Release.new
mv Release.new Release

fingerprint=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr:/ { print $10; exit }')
test -n "$fingerprint" || {
	echo "No secret signing key found in GNUPGHOME" >&2
	exit 1
}
gpg --batch --yes --pinentry-mode loopback --default-key "$fingerprint" --clearsign -o InRelease Release
gpg --batch --yes --pinentry-mode loopback --default-key "$fingerprint" --armor --detach-sign -o Release.gpg Release

# Verify with only the public key users will trust.
verify=$(mktemp -d)
trap 'rm -rf -- "$verify"' EXIT
gpg --batch --no-default-keyring --keyring "$verify/trusted.gpg" --import "$pubkey" 2>/dev/null
gpgv --keyring "$verify/trusted.gpg" Release.gpg Release
gpgv --keyring "$verify/trusted.gpg" InRelease >/dev/null

cp -- "$pubkey" luma-launcher.asc
cp -- "$here/../packaging/apt/index.html" index.html
echo "Built signed apt repository: ${#debs[@]} package file(s) in $dir"
