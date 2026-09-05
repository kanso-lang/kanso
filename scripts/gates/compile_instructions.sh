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
# ONE ROW, ONE VALUE. Ruled 2026-09-05, and it retired a per-chip table and a
# pinned pair. bench/compile_instructions_golden.txt holds a single
# `compile_instructions=` and this checks the measurement against it exactly.
# Every move is attributed to the change under test and handled by the ordinary
# ratchet, like every other counter in the tree.
#
# CONSISTENCY IS VERIFIED BY REPRODUCTION rather than by keying: the same build
# on any runner counts the same number. A run that disagrees is a reproduction
# failure, and it HALTS THE VEIN and is hunted to its source — the way the row's
# earlier drift turned out to be Rust's stack guard parsing /proc/self/maps at
# startup, and the answer was to force the measurement to be consistent. Pinning
# a second value, or recording the difference as a mode, is what that ruling
# forbids.
#
# The evidence it was decided on: eight within-binary sittings across two
# binaries, two vendors and four CPU generations, agreeing to the instruction
# and none disagreeing, at a cost of eight red CI rounds spent adding rows.
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
# A host the golden does not name still MEASURES on CI, so the job log carries
# the sitting the refusal tells a reader to copy. scripts/gates/host_gate.sh
# carries the reasons; 3 means measure, print, and fail at the end. The build is
# what the golden names — a toolchain it does not name is not the same build, so
# its number is not a reproduction of anything.
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

# And which silicon is about to count it. Not because the row is keyed by it —
# it is not, one row holds one value — but because a reproduction failure is
# hunted from the job log, and the first question about two numbers from one
# build is what they were counted on. scripts/gates/dispatch.sh carries the
# reasons the pool is worth naming.
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
# NO ASLR KNOB, and the reason is a measurement rather than an omission. The
# row read two values 508 apart and `setarch -R` was tried against them on the
# ruling of 2026-09-03. It changed nothing twice over: on the container it read
# 42,235,790 against 42,235,790 without it, and on CI a runner counted the same
# 41,832,275 with it as another counted without it. valgrind assigns the
# client's address space itself, so host randomization does not reach the count.
#
# What the 508 actually was: two binaries. sha 55fb850296d1 counted 41,831,767
# and sha de5bfab22fbd counts 41,832,275, and the comment below on compile_sample
# already names the binary as a cause the cpu key cannot see.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.compile ./kanso check lib/json \
    >/dev/null 2>/dev/null
)
# THE COMPILER'S OWN WORK, not the whole process. `kanso::main` inclusive is
# everything the compiler does, and that INCLUDES every
# libc call the compiler makes — so this is not the "stop counting glibc" that
# 2026-09-03 ruled out, and the difference is the whole point. What it drops is
# the 465,122 instructions ABOVE that frame: the loader mapping five shared
# objects, and Rust placing its stack guard, which parses /proc/self/maps and
# therefore moves with where the linker happened to put things.
#
# HOW MUCH IT BUYS, and it is not everything. Measured 2026-09-04 on seven
# binaries whose sources differ only in code or data nothing reaches. The
# `program` column is `std::rt::lang_start::{{closure}}` inclusive, which is
# what these seven were read with; `kanso::main` sat exactly 10 below it on
# every profile retained, and reads 41,878,949 on the baseline built with the
# anchor pinned. The spans are the same either way.
#
#   variant           .text     row         maps     program
#   baseline          2550854   42,344,081  112,580  41,878,959
#   +50 dead fns      2552534   42,348,024  114,845  41,879,987
#   +200 dead fns     2558486   42,347,128  112,586  41,879,361
#   +400 dead fns     2565174   42,348,044  110,341  41,879,922
#   +3 KiB .bss       2550854   42,346,221  114,720  41,878,959
#   +64 KiB .bss      2550854   42,346,221  114,720  41,878,959
#   +64 KiB .rodata   2550854   42,344,099  112,598  41,878,959
#
# The row spans 3,963 across those seven and the program frame spans 1,028, so
# the split takes about three quarters of the layout term. The residue is real
# and it comes from .text: 7,632 bytes of code no execution reaches moves the
# program frame 402, and the movement is not monotone in .text. Data-only
# changes leave the frame identical to the instruction, which is what the
# earlier four-binary reading saw — .bss and .rodata are the cases where the
# drop is total, and .text is not one of them.
#
# So a difference near a thousand on this row is not evidence on its own.
# Build the pair and read them.
#
# Startup work scales with what is loaded, so the one compiler change that can
# move the dropped half is growing a dependency — one more shared object was
# measured at 32,090. bench/compile_libraries_golden.txt watches that by name,
# which is a better answer than a number moving inside the layout noise.
#
# WHY THE ANCHOR IS OURS. This read `std::rt::lang_start::{{closure}}` for one
# round and CI refused: that frame is the standard library's, the runners'
# toolchain does not emit it, and a name std owns can move under a version bump
# without anybody here touching a line. `kanso::main` is this crate's, carries
# an `inline(never)` in src/main.rs that says so, and differed from the closure
# by exactly 10 instructions on all four profiles measured on 2026-09-04 — so
# the spans below are the same either way.
#
# A REFUSAL rather than an empty value when the frame is missing, and the
# profile PRINTED FIRST so one job log says what it does contain. The round
# that made this necessary refused with nothing to read: `scripts/perf_record`
# had already spent a day reporting `missing index 2` against its own reader,
# and a refusal that cannot be diagnosed from its own output repeats it.
if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi
echo "=== the profile's top frames, inclusive"
callgrind_annotate --inclusive=yes --threshold=99 /tmp/cg.compile 2>&1 | head -30
own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.compile 2>/dev/null \
      | awk '/kanso::main/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
case "$own" in
  '' | *[!0-9]*)
    echo "::error::the profile carries no kanso::main frame, so the compiler's"
    echo "::error::own work cannot be read out of it. The listing above is what"
    echo "::error::it does carry. A toolchain that folded that frame away stops"
    echo "::error::this gate rather than pinning whatever the pipe returned."
    exit 1
    ;;
esac
printf 'compile_instructions=%s\n' "$own" > compile_ir_got.txt

# ONE GREPPABLE LINE PER RUN, pairing the value with the two things that move
# it. Both are now established rather than suspected. The cpu moves it about
# 5,124: identical sources from f8fd75cb on read 41,500,974 and 41,495,850 on
# the pool, each value two or three times. The binary moves it too, and not
# only when the front end changes — src/runtime.c is include_str!'d into the
# compiler, so editing the RUNTIME moves the compiler's bytes; held on one
# chip with both shas printed, kanso#1226's runtime change moved it -5,621.
#
# Neither is keyed out — the row is one value and both of those causes have to
# reproduce. This line is how a disagreement is read: same sha with different
# rows is a reproduction failure and halts the vein; different shas is the
# change under test until the pair is built and both are read.
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

# WHETHER IT LANDED ON THE ROW. One row, one value, compared exactly — the
# ordinary ratchet every other counter in the tree gets.
want=$(sed -n 's/^compile_instructions=//p' "$golden")
got=$(sed -n 's/^compile_instructions=//p' compile_ir_got.txt)
case "$want" in
  '' | *[!0-9]*)
    echo "::error::$golden carries no single compile_instructions= value, so"
    echo "::error::there is nothing to compare the sitting above against. This"
    echo "::error::vein is read by an exact compare and holds exactly one value."
    exit 1
    ;;
esac

# A toolchain the golden does not name is not the same build, so its number
# reproduces nothing and is not read against the row. It is still printed above.
if [ "$host" -eq 3 ]; then
  echo "::error::the sitting above was counted on a toolchain $golden does not"
  echo "::error::name, so it is not a reproduction of the recorded build and"
  echo "::error::says nothing about the value. Name the toolchain in $golden,"
  echo "::error::with the measured-on lines, before reading a row off it."
  exit 1
fi

if [ "$got" = "$want" ]; then
  echo "compile_instructions: $got, on the row"
  exit 0
fi

echo "::error::compile_instructions counted $got against $want in $golden,"
echo "::error::a move of $((got - want)). Exactly one of two things is true,"
echo "::error::and they are settled differently."
echo "::error::"
echo "::error::(1) THE CHANGE UNDER TEST MOVED IT. Ordinary ratchet: regenerate"
echo "::error::    $golden, and say in design/compiler-log.md which way it went"
echo "::error::    and why, which is the sentence the trend gate reads. Note"
echo "::error::    that src/runtime.c and lib/*.kso are include_str!'d into the"
echo "::error::    compiler, so a runtime or library edit moves this row with"
echo "::error::    the front end untouched."
echo "::error::"
echo "::error::(2) THE SAME BUILD COUNTED TWO NUMBERS. That is a REPRODUCTION"
echo "::error::    FAILURE. It halts this vein and is hunted to its source --"
echo "::error::    never pinned as a second value, and never recorded as a mode."
echo "::error::    The last one was Rust's stack guard parsing /proc/self/maps"
echo "::error::    at startup, and the answer was to anchor the count at"
echo "::error::    kanso::main so the measurement stopped depending on it."
echo "::error::"
echo "::error::The compile_binary sha256 and compile_sample lines above are"
echo "::error::where the hunt starts: one sha counting two rows is (2); two"
echo "::error::shas is (1) until the pair is built and both are read."
exit 1
