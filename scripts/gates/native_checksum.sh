#!/bin/sh
set -e
cargo build --release
./target/release/kanso run bench/make_jsonbench
./target/release/kanso build bench/jsonbench --release
out=$(./jsonbench)
echo "$out"
echo "$out" | grep -q "checksum 24000" \
  || { echo "::error::native json decoder produced the wrong checksum"; exit 1; }
