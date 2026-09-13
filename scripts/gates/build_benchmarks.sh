#!/bin/sh
# What the counter gates beside this one read. Not a gate itself.
set -e
cargo build --release
./target/release/kanso run bench/make_jsonbench
# TWO binaries per benchmark. The emitted fast paths ask `k_stats_on` before
# they take the shortcut, and a binary nobody is going to count does not need
# the question -- 26,667,341 instructions on the run program, 1.2193%. So the
# counting set is built first under `--counters` and moved aside, and the
# plain set is built second and keeps the bare name, because the instruction,
# machine-code and checksum gates all read the bare name and must read the
# binary that ships. deepbench and indexbench have no counter gate -- their
# rows live in the instructions vein -- so they are built once.
./target/release/kanso build bench/jsonbench --release --counters >/dev/null
mv jsonbench jsonbench-counters
./target/release/kanso build bench/encodebench --release --counters >/dev/null
mv encodebench encodebench-counters
./target/release/kanso build bench/oneshot --release --counters >/dev/null
mv oneshot oneshot-counters
./target/release/kanso build bench/basket --release --counters >/dev/null
mv basket basket-counters
./target/release/kanso build bench/widebench --release --counters >/dev/null
mv widebench widebench-counters
./target/release/kanso build bench/escapebench --release --counters >/dev/null
mv escapebench escapebench-counters
./target/release/kanso build bench/pendbench --release --counters >/dev/null
mv pendbench pendbench-counters
./target/release/kanso build bench/scanbench --release --counters >/dev/null
mv scanbench scanbench-counters
./target/release/kanso build bench/digestbench --release --counters >/dev/null
mv digestbench digestbench-counters
./target/release/kanso build bench/readbench --release --counters >/dev/null
mv readbench readbench-counters
./target/release/kanso build bench/livebench --release --counters >/dev/null
mv livebench livebench-counters
./target/release/kanso build bench/runbench --release --counters >/dev/null
mv runbench runbench-counters

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
./target/release/kanso build bench/livebench --release >/dev/null
./target/release/kanso build bench/runbench --release >/dev/null
