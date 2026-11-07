#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
cargo fmt --check
cargo clippy --locked --all-targets --all-features -- -D warnings
# Widget tests need a display; they skip themselves when none is available.
run=()
command -v xvfb-run >/dev/null && run=(xvfb-run -a)
"${run[@]}" cargo test --locked
cargo build --locked --release
bash scripts/test-install.sh
