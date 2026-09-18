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
# AND THE BINARY IS BUILT HERE, because this script stages one it did not
# make. `cp ./target/release/kanso` copies whatever that path holds, and a
# worktree's target directory holds whatever was last built in it -- which on
# 2026-09-18 was a compiler from before 3ee41dcf, the 2026-09-16 fix
# that moved the interpreter's tables off std's randomly-seeded hasher. Four
# runs staged out of that box read the interpreted anchor 3,100,448 apart with
# `sip::Hasher::write` live at 187,455,582, and the spread went into a log
# entry and an open pull request as a property of the current tree.
#
# Four runs of a release build of the same commit read ONE value with no
# SipHash frame at all. A build of `3ee41dcf^` handed the same corpus reads
# 2,649,396,935 / 2,654,191,562 / 2,650,473,347 / 2,653,900,730 and puts
# SipHash at 187,455,582 -- the stale reading's own figure, to the
# instruction.
#
# It is the rule `all_compile.sh` already carries for the artifacts its gates
# read: a measurement script builds what it measures rather than trusting what
# is lying about. `all_counters.sh` opens with this same line for the same
# reason. In CI it is a no-op, because the workflow has already built; on a
# box where somebody is measuring by hand it is the difference between the
# tree's number and some other tree's.
cargo build --release
box=/tmp/kanso-compile-ir
rm -rf "$box"
mkdir -p "$box"
cp -R lib "$box/lib"
find "$box/lib" -name '*_test.kso' -delete
cp ./target/release/kanso "$box/kanso"
cp -R bench/compile_corpus "$box/compile_corpus"
# And the entry corpus, for the same reasons. `kanso check <dir>` is the module
# path and `kanso check <file>` is the entry path, so the two corpora measure
# two different compiles and both are staged here rather than read out of the
# checkout — the path length moves the count either way.
cp -R bench/entry_corpus "$box/entry_corpus"
# And the library corpus, the third of the three. `kanso check` routes a single
# file by content: bare statements make it an entry, definitions alone make it a
# library, and those take different functions with different pass orders. Named
# to the same length as compile_corpus so the one term that is not the compiler
# — the path the count tracks, about 160 instructions a character — is the same
# for all three.
cp -R bench/library_corpus "$box/library_corpus"
# And the interpreted corpus, which is not a compile at all: `kanso run
# --interp` parses, infers and then EXECUTES in the same process, so the box
# holds it for the interpreter's own two veins. Staged here beside the other
# three because the path length moves an instruction count whatever the verb
# reading it -- about 160 instructions a character, the measurement the header
# of this file carries.
cp -R bench/interp_corpus "$box/interp_corpus"

# And the start-up corpus, which is one `print` and exists to be small. The
# other three corpora measure what compiling a body of code costs; this one
# measures what a `kanso play` costs before it has any work to do, which is
# the term `kanso test` pays on every invocation and never pays once in
# production. A one-line program is the whole point: what is left is start-up.
cp -R bench/startup_corpus "$box/startup_corpus"
