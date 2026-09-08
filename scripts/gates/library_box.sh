#!/bin/sh
# The compile corpus and the library it imports, staged without the library's
# tests, at a fixed path.
#
# THE WORKLOAD IS NAMED, NOT INHERITED. Since 2026-09-08 the three compile
# gates check bench/compile_corpus rather than lib/json, because a term
# measured on a library moves when that library changes its imports:
# kanso#1291 dropped std/list from lib/json and the compile rows halved with
# the compiler byte-identical. The corpus names what it imports, so a row
# moves by a compiler change or by an edit to the corpus itself. It lives
# under bench/ rather than lib/ because a benchmark is not the library, and
# that is why it needs a staging line of its own below.
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
cp -R bench/compile_corpus "$box/compile_corpus"
cp -R bench/entry_corpus "$box/entry_corpus"

# TWO CORPORA, TWO PATHS. bench/compile_corpus is a directory module and
# `kanso check` on it goes through compile_module_inner; bench/entry_corpus is
# a FILE and `kanso check` on it goes through compile_parsed_entry. The two
# order their passes differently, so a change made on one path and left on the
# other reads as green everywhere. Staged from the same fixed path for the same
# reason the first one is: the count tracks the length of the directory the
# compiler runs in.
