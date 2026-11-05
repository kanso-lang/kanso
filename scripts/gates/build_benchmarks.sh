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
# The run program once more at the dev tier, which is what `kanso run` and a
# bare `kanso build` hand a developer. instructions.sh counts it as the
# `runbench_dev` row, the development side's run-speed term. Built before the
# release set, which writes `runbench` again.
./target/release/kanso build bench/runbench >/dev/null
mv runbench runbench_dev
for b in $(sh scripts/gates/benchmarks.sh); do
  ./target/release/kanso build bench/$b --release >/dev/null
done
