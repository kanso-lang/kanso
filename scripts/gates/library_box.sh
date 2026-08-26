#!/bin/sh
# The library, staged without its tests, at a fixed path.
#
# Two instrument faults, one staging step.
#
# THE TESTS ARE NOT THE LIBRARY. `kanso check lib/json` compiles lib/json's
# TEST file too, so every dependency the suite imports enters the program the
# compile goldens measure. That surfaced on 2026-08-25: obeying gavels 1b and
# 24 moved json's assertions onto `std/testing`, and the single added import
# cost the LIBRARY's golden two fixpoint rounds, a hundred expression visits,
# 5,989 peak bytes and a million retired instructions. None of it was the
# library getting more expensive to compile. A row that moves when a test file
# changes its imports is answering a question nobody asked.
#
# THE PATH MOVES THE COUNT. Retired instructions track the length of the
# directory the compiler runs in — about 160 per character, because the
# absolute path is copied and walked — so a row read from the checkout pins
# the clone rather than the compiler. compile_instructions.sh already staged
# for this reason; allocations and peak bytes are measured from the same fixed
# path now so all three veins answer for the same program.
set -e
box=/tmp/kanso-compile-ir
rm -rf "$box"
mkdir -p "$box"
cp -R lib "$box/lib"
find "$box/lib" -name '*_test.kso' -delete
cp ./target/release/kanso "$box/kanso"
