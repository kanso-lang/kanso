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
# A host neither file names still MEASURES on CI, so the job log carries the
# sitting the refusal tells a reader to copy. scripts/gates/host_gate.sh carries
# the reasons; 3 means measure, print, and fail at the end. Both files are asked,
# and the worse answer wins — a stop from either is a stop.
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi
tablehost=0
sh scripts/gates/host_gate.sh "$table" || tablehost=$?
if [ "$tablehost" -ne 0 ] && [ "$tablehost" -ne 3 ]; then
  exit "$tablehost"
fi
if [ "$tablehost" -eq 3 ]; then
  host=3
fi

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
# THE COMPILER'S OWN WORK, not the whole process. `std::rt::lang_start::
# {{closure}}` inclusive is everything `main` does, and that INCLUDES every
# libc call the compiler makes — so this is not the "stop counting glibc" that
# 2026-09-03 ruled out, and the difference is the whole point. What it drops is
# the 465,122 instructions ABOVE that frame: the loader mapping five shared
# objects, and Rust placing its stack guard, which parses /proc/self/maps and
# therefore moves with where the linker happened to put things.
#
# HOW MUCH IT BUYS, and it is not everything. Measured 2026-09-04 on seven
# binaries whose sources differ only in code or data nothing reaches:
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
# A REFUSAL rather than an empty value when the frame is missing. A toolchain
# that renames it must stop this gate, not pin whatever the pipe returned:
# scripts/perf_record read welfare's score as a field of a line it never
# checked, and reported `missing index 2` against its own reader for a day.
if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi
own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.compile 2>/dev/null \
      | awk '/lang_start::\{\{closure\}\}/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
case "$own" in
  '' | *[!0-9]*)
    echo "::error::the profile carries no std::rt::lang_start::{{closure}} frame,"
    echo "::error::so the compiler's own work cannot be read out of it. A"
    echo "::error::toolchain that renamed that frame stops this gate rather than"
    echo "::error::pinning whatever the pipe returned."
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
if [ "$host" -eq 3 ]; then
  echo "::error::the row above is this runner's sitting on a host $table does"
  echo "::error::not name, so it is NOT read against any recorded chip. Copy it"
  echo "::error::into $table and $golden together with the measured-on lines."
  exit 1
fi

sh scripts/gates/compile_ir_row.sh "$table" "$golden" \
  "$(sh scripts/gates/dispatch.sh key)" \
  "$(sed -n 's/^compile_instructions=//p' compile_ir_got.txt)"
