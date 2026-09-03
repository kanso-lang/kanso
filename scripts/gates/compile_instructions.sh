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
# THE ROW IS KEYED BY SILICON. bench/compile_instructions_by_cpu.txt holds one
# value per chip and is what this checks against;
# bench/compile_instructions_golden.txt holds its first row again as a bare
# number, for welfare and the trend gate, and this checks the two agree.
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
table=bench/compile_instructions_by_cpu.txt
sh scripts/gates/measured_on.sh "$golden"
sh scripts/gates/measured_on.sh "$table"

# And which silicon is about to count it, which decides WHICH ROW this run is
# read against. The pool is at least four CPUs and glibc picks its memcpy by
# ifunc, so one number cannot be right on all of them.
# scripts/gates/dispatch.sh carries the reasons.
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

# WHY THE TUNABLES ARE PINNED. Emptying the environment was not enough: on one
# unchanged binary this row sat in two stable clusters 5,064 apart, and the same
# family and model produced both. The two profiles differ in _int_malloc
# (1,551,398 against 1,554,268) and in __memcmp_avx2_movbe (1,346,206 against
# 1,347,513) — the same memcmp implementation counting differently, which is an
# alignment difference, which is a heap-layout difference upstream of it.
#
# glibc sizes its malloc and string thresholds from the cache sizes it reads out
# of the CPU, so two machines of one model with different caches lay the heap
# out differently. Pinning the tunables makes every run take the same path on
# every host. It measures a configuration no user runs under, which is the
# price: this row is for comparing the compiler against itself, and a number
# that cannot be compared measures nothing at all.
tune=glibc.cpu.x86_data_cache_size=0x8000
tune=$tune:glibc.cpu.x86_shared_cache_size=0x2000000
tune=$tune:glibc.cpu.x86_non_temporal_threshold=0x1800000
tune=$tune:glibc.cpu.x86_rep_movsb_threshold=0x840
tune=$tune:glibc.cpu.x86_rep_stosb_threshold=0x800
tune=$tune:glibc.malloc.arena_max=1
tune=$tune:glibc.malloc.mmap_threshold=131072
tune=$tune:glibc.malloc.trim_threshold=131072
tune=$tune:glibc.malloc.top_pad=131072
tune=$tune:glibc.malloc.tcache_count=7
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.compile ./kanso check lib/json \
    >/dev/null 2>/dev/null
)
printf 'compile_instructions=%s\n' \
  "$(grep -o '^summary: [0-9]*' /tmp/cg.compile | tr -dc 0-9)" > compile_ir_got.txt

# ONE GREPPABLE LINE PER RUN, pairing the value with the two things that move
# it. Both are now established rather than suspected. The cpu moves it about
# 5,124: identical sources from f8fd75cb on read 41,500,974 and 41,495,850 on
# the pool, each value two or three times. The binary moves it too, and not
# only when the front end changes — src/runtime.c is include_str!'d into the
# compiler, so editing the RUNTIME moves the compiler's bytes; held on one
# chip with both shas printed, kanso#1226's runtime change moved it -5,621.
#
# The table keys out the first. This line is what would catch a third cause,
# and is what caught the second: same cpu and same sha with different rows
# would mean something is moving that neither the key nor the diff can see.
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

# Which row this run is read against, and whether it landed on it.
# scripts/gates/compile_ir_row.sh is a separate file so its four refusals can
# be watched failing without a callgrind run each time.
sh scripts/gates/compile_ir_row.sh "$table" "$golden" \
  "$(sh scripts/gates/dispatch.sh key)" \
  "$(sed -n 's/^compile_instructions=//p' compile_ir_got.txt)"
