#!/bin/sh
# What the FRONT END costs to run, counted rather than timed.
#
# bench/instructions_golden.txt counts the benchmarks — the programs the
# compiler produces. Nothing counted the compiler itself, and on 2026-08-24 a
# change took `kanso check lib/json` from 90.9 million retired instructions to
# 67.2 million with every gate in the tree reporting nothing: allocations
# identical, rounds identical, visits identical, peak inside its band. A
# quarter of the work went away silently, and a quarter coming back would have
# been just as quiet.
#
# THE ROW IS MEASURED IN A BOX, not in the checkout, and the box holds the
# library without its tests — scripts/gates/library_box.sh says why for both.
# The short version: the count moves with the length of the directory the
# compiler runs in, and a test file's imports are not the library's cost.
#
# The environment is emptied for the same reason instructions.sh empties it:
# the kernel copies the environment block onto the new process's stack and
# libc walks it before main, so a run id that gained a digit reads as work
# nobody wrote.
set -e
golden=bench/compile_instructions_golden.txt
sh scripts/gates/measured_on.sh "$golden"

sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir
(
  cd "$box"
  env -i PATH=/usr/bin:/bin valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.compile ./kanso check lib/json \
    >/dev/null 2>/dev/null
)
printf 'compile_instructions=%s\n' \
  "$(grep -o '^summary: [0-9]*' /tmp/cg.compile | tr -dc 0-9)" > compile_ir_got.txt

# The profile is on disk either way, and where the front end's work sits is the
# question every one of these moves turns on. Printed rather than summarised,
# because a step summary cannot be read back from the job log.
if command -v callgrind_annotate >/dev/null; then
  echo "=== where the front end's work is"
  callgrind_annotate --threshold=90 /tmp/cg.compile 2>&1 | head -40
fi

grep -v '^#' "$golden" > compile_ir_want.txt
diff compile_ir_want.txt compile_ir_got.txt || {
  echo "::error::the work the FRONT END does changed. A rise is a regression"
  echo "::error::to explain and a fall is a win to bank — say which in"
  echo "::error::design/compiler-log.md and regenerate $golden."
  echo "::error::This is the dimension that stayed silent while a quarter of"
  echo "::error::the compiler's work went away: allocations, rounds, visits"
  echo "::error::and peak were all identical across that change."
  exit 1
}
