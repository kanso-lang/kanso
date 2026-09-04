#!/bin/sh
# What the counter gates beside this one read. Not a gate itself.
set -e
cargo build --release
./target/release/kanso run bench/make_jsonbench
./target/release/kanso build bench/jsonbench --release
./target/release/kanso build bench/encodebench --release >/dev/null
./target/release/kanso build bench/oneshot --release >/dev/null
./target/release/kanso build bench/basket --release >/dev/null
./target/release/kanso build bench/widebench --release >/dev/null
./target/release/kanso build bench/deepbench --release >/dev/null
./target/release/kanso build bench/escapebench --release >/dev/null
./target/release/kanso build bench/pendbench --release >/dev/null
./target/release/kanso build bench/indexbench --release >/dev/null
./target/release/kanso build bench/scanbench --release >/dev/null
./target/release/kanso build bench/digestbench --release >/dev/null
./target/release/kanso build bench/readbench --release >/dev/null
