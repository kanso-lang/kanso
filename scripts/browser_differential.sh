#!/bin/sh
# Differential test for the playground in a real browser: every golden-corpus
# program the page compiles and runs on runtime.c must produce byte-identical
# (status, output) to the native engine. A disagreement is excused only where
# tests/golden/native_route_gaps.txt records the text the page answers instead.
set -e
cd "$(dirname "$0")/.."
cargo build --release
sh scripts/build_wasm.sh
sh scripts/build_runtime_wasm.sh
./target/release/kanso run scripts/browser_differential_run
