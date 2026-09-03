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

# And which silicon is about to count it. This never refuses — the runner pool
# holds at least four CPUs — it prints, and is asked a question below only if
# the row has actually moved. scripts/gates/dispatch.sh carries the reasons.
sh scripts/gates/dispatch.sh name

sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir

# WHICH BINARY COUNTED IT. Cargo builds are not bit-reproducible by default,
# and a binary whose data and bss differ starts the heap at a different break.
# That moves how much work malloc does to service an identical request
# sequence without moving a single instruction the compiler executes — which
# is exactly the shape this row's variance has: seven readings, four distinct
# values, every kanso symbol identical to the instruction across a pair of
# them and only glibc's allocator moving. Printed on every run, green or red,
# so a hash can be paired with the value the row landed on.
printf 'compile_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|data|bss)[ \t]/ { printf "compile_binary %s=%s\n", $1, $2 }'

(
  cd "$box"
  env -i PATH=/usr/bin:/bin valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.compile ./kanso check lib/json \
    >/dev/null 2>/dev/null
)
printf 'compile_instructions=%s\n' \
  "$(grep -o '^summary: [0-9]*' /tmp/cg.compile | tr -dc 0-9)" > compile_ir_got.txt

# ONE GREPPABLE LINE PER RUN, so the three things that could be moving this row
# can be told apart without reading a job log by hand. The row has taken four
# values on a front end nothing touched; two candidate causes are live and
# nothing yet separates them. The cpu is one — two runs differ in cpuid word
# and L2 size. The binary is the other, and it is not idle: src/runtime.c is
# include_str!'d into the compiler, so a change to the RUNTIME moves the
# compiler's bytes and with them where its heap starts, without altering a
# single instruction the front end executes.
#
# Neither can be ruled in from one sample. Three runs with this line in them
# can: same cpu and same sha with different rows means it is neither, same cpu
# and different sha tracking the row means it is the binary, and same sha on
# different cpus tracking the row means it is the silicon.
printf 'compile_sample cpu="%s" sha=%.12s row=%s\n' \
  "$(sh scripts/gates/dispatch.sh name | sed -n 's/^silicon: //p')" \
  "$(sha256sum "$box/kanso" | cut -d' ' -f1)" \
  "$(sed -n 's/^compile_instructions=//p' compile_ir_got.txt)"

# The profile is on disk either way, and where the front end's work sits is the
# question every one of these moves turns on. Printed rather than summarised,
# because a step summary cannot be read back from the job log.
if command -v callgrind_annotate >/dev/null; then
  echo "=== where the front end's work is"
  callgrind_annotate --threshold=90 /tmp/cg.compile 2>&1 | head -40
fi

grep -v '^#' "$golden" > compile_ir_want.txt
diff compile_ir_want.txt compile_ir_got.txt || {
  # The row moved. glibc dispatches memcpy and its neighbours by CPU feature
  # and the compiler runs them, so ask whether this is even the same silicon
  # before calling it a regression. Answer 2 is "nothing recorded to compare
  # against", which gates exactly as this always did.
  sh scripts/gates/dispatch.sh differs bench/dispatch.txt && silicon=0 \
    || silicon=$?
  if [ "$silicon" -eq 1 ]; then
    echo "::error::and this is NOT the silicon the row was counted on, so the"
    echo "::error::dispatch above may account for some of the move. It does not"
    echo "::error::excuse it: re-run until the job lands on the recorded cpu,"
    echo "::error::and say in the pull request which way it went."
  fi
  echo "::error::the work the FRONT END does changed. A rise is a regression"
  echo "::error::to explain and a fall is a win to bank — say which in"
  echo "::error::design/compiler-log.md and regenerate $golden."
  echo "::error::This is the dimension that stayed silent while a quarter of"
  echo "::error::the compiler's work went away: allocations, rounds, visits"
  echo "::error::and peak were all identical across that change."
  exit 1
}
