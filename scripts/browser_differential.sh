#!/bin/sh
# Differential test for the browser wasm backend: every golden-corpus program
# it compiles must produce byte-identical (status, output) to the native
# engine. A disagreement is excused only where tests/golden/wasm_gaps.txt
# records the text the wasm engine answers instead.
set -e
cd "$(dirname "$0")/.."
cargo build --release
sh scripts/build_wasm.sh
./target/release/kanso run scripts/browser_differential_run
