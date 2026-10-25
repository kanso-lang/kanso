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
# binary that ships. A benchmark marked `plain` in bench/benchmarks.txt has
# no counter gate -- its rows live in the instructions vein -- so it is built
# once.
for b in $(sh scripts/gates/benchmarks.sh counted); do
  ./target/release/kanso build bench/$b --release --counters >/dev/null
  mv $b $b-counters
done
for b in $(sh scripts/gates/benchmarks.sh); do
  ./target/release/kanso build bench/$b --release >/dev/null
done
