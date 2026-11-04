#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
cargo fmt --check
cargo clippy --offline --all-targets --all-features -- -D warnings
cargo test --offline
cargo build --offline --release
cargo test --offline --release --test glib_variant
bash scripts/test-install.sh
