#!/usr/bin/env bash
set -euo pipefail
cd -- "$(dirname -- "$0")/.."
cargo bench --bench search
hyperfine --warmup 2 'target/release/luma-launcher --version' 'target/release/luma-launcher --diagnostics'
