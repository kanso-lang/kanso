#!/bin/sh
# What the compiler LOADS, which is the half of process startup that a
# compiler change can actually move.
#
# WHY THIS EXISTS. compile_instructions.sh counts the work the compiler does,
# from `std::rt::lang_start::{{closure}}` down, rather than the whole process.
# Everything before that frame is the loader mapping shared objects, relocating
# them, and placing Rust's stack guard — 465,122 instructions of it — and none
# of that is the compiler. Measured on 2026-09-04 over seven binaries whose
# sources differ only in code or data nothing reaches, the whole-process row
# spans 3,963 and the compiler's own frame spans 1,028.
#
# WHAT COUNTING THE PROGRAM GOES BLIND TO, and what this file is for. Startup
# work is proportional to what gets loaded, so the one compiler change that
# moves it is growing a dependency — one more shared object was measured at
# 32,090. That is a real thing to catch and it is better caught by name: a
# number that moved 32,090 among a thousand of linker luck is a puzzle, and
# `libfoo.so.1 appeared` is an answer.
#
# So the split is deliberate. The instruction gate watches the compiler's own
# work, where the layout term is about a thousand; this watches what it links,
# where the answer is a list of names and the layout term is nothing at all.
set -e
golden=bench/compile_libraries_golden.txt
cargo build --release
ldd ./target/release/kanso | awk '{ n = $1; sub(/.*\//, "", n); print n }' \
  | sort > compile_libraries_got.txt
grep -v '^#' "$golden" > compile_libraries_want.txt
diff compile_libraries_want.txt compile_libraries_got.txt || {
  echo "::error::the compiler's shared libraries moved. An ADDED one costs"
  echo "::error::startup work the instruction gate deliberately does not"
  echo "::error::count — about 32,090 for one — so this is where that shows"
  echo "::error::up. A REMOVED one is a win to say out loud. Either way,"
  echo "::error::name the library and why it is there, and regenerate"
  echo "::error::$golden."
  exit 1
}
