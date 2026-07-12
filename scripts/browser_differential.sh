#!/bin/sh
# Differential test for the browser wasm backend: every golden-corpus program
# it compiles must produce byte-identical (status, output) to the native
# engine. Backend fallbacks are reported (with reasons) but do not fail.
set -e
cd "$(dirname "$0")/.."
cargo build --release
sh scripts/build_wasm.sh
python3 scripts/browser_differential.py
