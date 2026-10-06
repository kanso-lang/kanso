#!/bin/sh
# What the BROWSER engine costs: the playground compiling a program inside the
# tab and running the module it emits, counted rather than timed.
#
#   sh scripts/gates/browser_cost.sh
#
# Ruled 2026-10-06: welfare weighs every environment kanso runs in, and none at
# zero. tests/browser_cost.rs loads docs/kanso.wasm into wasmi with fuel
# metering on, compiles bench/interp_corpus with it, runs what comes out, and
# compares four rows with bench/browser_golden.txt. Fuel is spent per executed
# wasm instruction and the peaks are the toolchain allocator's own tally, so
# the same artifact reads the same number on every run and every machine.
#
# THE ARTIFACT IS BUILT HERE, because no gate may read a file it did not build:
# the compile sweep learned that on 2026-09-06 from three-minute-old binaries.
# docs/kanso.wasm is not committed, and a stale one would price another tree.
#
# The rustc that builds it decides every row -- the wasm target is the same on
# every host, so the host does not -- which is why the golden names its rustc
# and the spec refuses to compare against another one. Under CI it measures,
# prints and fails, the way host_gate.sh does for the compile veins.
set -e
rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true
rm -f docs/kanso.wasm
sh scripts/build_wasm.sh
cargo test --release --test browser_cost
