#!/usr/bin/env bash
# Prove a stock apt accepts a repository built by build-apt-repo.sh: the signature
# verifies against the public key, the package is offered, and a tampered
# repository is rejected. Uses private apt directories, so no root is needed.
#
#   bash scripts/test-apt-repo.sh <repo-dir> <public-key.asc>
set -euo pipefail

repo=$(realpath "${1:?usage: test-apt-repo.sh <repo-dir> <public-key.asc>}")
key=$(realpath "${2:?usage: test-apt-repo.sh <repo-dir> <public-key.asc>}")
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT

apt_opts() { # <sources.list> <state-dir>
	printf '%s\n' \
		-o "Dir::Etc::sourcelist=$1" -o "Dir::Etc::sourceparts=/dev/null" \
		-o "Dir::State=$2" -o "Dir::Cache=$work/cache" \
		-o "Dir::State::status=/var/lib/dpkg/status" -o "Debug::NoLocking=1" \
		-o "APT::Sandbox::User="
}
run_apt() { # <tool> <sources.list> <state-dir> args...
	local tool=$1 list=$2 state=$3
	shift 3
	local opts
	mapfile -t opts < <(apt_opts "$list" "$state")
	mkdir -p "$state/lists/partial" "$work/cache/archives/partial"
	"$tool" "${opts[@]}" "$@"
}

echo "deb [signed-by=$key] file:$repo ./" >"$work/good.list"
run_apt apt-get "$work/good.list" "$work/state-good" update
run_apt apt-cache "$work/good.list" "$work/state-good" policy luma-launcher | tee "$work/policy.txt"
grep -Eq 'Candidate: [0-9]' "$work/policy.txt"
echo "OK: apt verified the signature and offers luma-launcher"

# A repository whose Packages file was altered after signing must be refused.
cp -r -- "$repo" "$work/tampered"
echo "# tampered" >>"$work/tampered/Packages"
gzip -9nkf "$work/tampered/Packages"
echo "deb [signed-by=$key] file:$work/tampered ./" >"$work/bad.list"
if run_apt apt-get "$work/bad.list" "$work/state-bad" update >"$work/bad.out" 2>&1; then
	if ! grep -Eqi 'hash sum mismatch|failed|error' "$work/bad.out"; then
		echo "FAIL: a tampered repository was accepted" >&2
		cat "$work/bad.out" >&2
		exit 1
	fi
fi
echo "OK: a tampered repository is rejected"
