#!/bin/sh
# What an INTERPRETED run costs, counted rather than timed.
#
# The three compile gates run `kanso check`, which stops before codegen, and
# startup_instructions.sh runs `kanso play` on a one-line program, which is
# almost all loader and backend. This one runs `kanso run --interp` on a corpus
# that decodes a document it built itself, six times over, so what dominates is
# the interpreter executing rather than the front end deciding. It is the
# interpreted engine's SPEED row; its memory rows are interp_memory.sh and its
# start-up is startup_instructions.sh.
#
# WHY IT EXISTS. Clay ruled on 2026-09-16 that the objective becomes a
# development welfare, a production welfare and a meta over them, and he named
# the interpreted engine's own order: "start-time is vastly more important than
# speed which is more important than memory usage." This is the middle of the
# three. It is an exact vein of its own and not yet an objective term, the way
# `.text` is under the 2026-09-05 ruling; the objective takes it when the model
# splits.
#
# THE CORPUS BUILDS ITS OWN INPUT, on purpose. Every other benchmark in this
# tree reads bench/large.json, and a row that reads a file is a row that moves
# when the file does. This one interpolates a document of 220 objects and
# decodes it six times, so the workload is a property of the corpus alone and
# the box needs nothing staged beside it.
#
# EVERYTHING THIS SHARES WITH THE MODULE ROW IS EXPLAINED THERE, at length and
# with the measurements behind it: why the environment is emptied, why the
# glibc tunables are pinned, why the count is anchored at a frame rather
# than at the process, why there is no ASLR knob, why the binary's sha and
# section sizes are printed on every run, and why one row holds exactly one
# value. Read compile_instructions.sh; none of it is restated here.
set -e
golden=bench/interp_instructions_golden.txt
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

sh scripts/gates/dispatch.sh name

sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir

printf 'interp_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|data|bss)[ \t]/ { printf "interp_binary %s=%s\n", $1, $2 }'

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
# THE CORPUS IS NAMED RELATIVE TO THE BOX. The count tracks the length of the
# path the compiler is handed — about 160 instructions a character, because
# the absolute path is copied and walked — so `interp_corpus` and
# `/tmp/kanso-compile-ir/interp_corpus` are different numbers for the same
# run. library_box.sh carries the measurement.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.interp ./kanso run interp_corpus --interp \
    >/dev/null 2>/dev/null
)

if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi

echo "=== the profile's top frames, inclusive"
callgrind_annotate --inclusive=yes --threshold=99 /tmp/cg.interp 2>&1 | head -30

# THE ANCHOR IS THE INTERPRETER'S OWN THREAD, not `kanso::main`. `kanso run
# --interp` pins a one-gigabyte stack and runs the interpreter on a thread of
# its own, so `kanso::main` inclusive sees the front end and 1.8% of the run:
# on the first sitting it read 48,026,664 against 2,700,128,254 for the whole
# process. `run_interpreted_on_stack` is that thread's entry, it is not
# recursive, and it excludes the loader for the same reason the other gates
# exclude it.
own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.interp 2>/dev/null \
      | awk '/kanso::run_interpreted_on_stack/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
case "$own" in
  '' | *[!0-9]*)
    echo "::error::the profile carries no run_interpreted_on_stack frame, so"
    echo "::error::the interpreter's own work cannot be read out of it. The"
    echo "::error::listing above is what it does carry. A toolchain that"
    echo "::error::inlined that frame away stops this gate rather than pinning"
    echo "::error::whatever the pipe returned."
    exit 1
    ;;
esac

printf 'interp_instructions=%s\n' "$own" > interp_ir_got.txt
printf 'interp_sample cpu="%s" sha=%.12s row=%s\n' \
  "$(sh scripts/gates/dispatch.sh name | sed -n 's/^silicon: //p')" \
  "$(sha256sum "$box/kanso" | cut -d' ' -f1)" \
  "$(sed -n 's/^interp_instructions=//p' interp_ir_got.txt)"

echo "=== where the interpreted run's work is"
callgrind_annotate --threshold=90 /tmp/cg.interp 2>&1 | head -40

want=$(sed -n 's/^interp_instructions=//p' "$golden")
got=$(sed -n 's/^interp_instructions=//p' interp_ir_got.txt)
case "$want" in
  '' | *[!0-9]*)
    echo "::error::$golden carries no single interp_instructions= value, so"
    echo "::error::there is nothing to compare the sitting above against. This"
    echo "::error::vein is read by an exact compare and holds exactly one value."
    exit 1
    ;;
esac

if [ "$host" -eq 3 ]; then
  echo "::error::the sitting above was counted on a toolchain $golden does not"
  echo "::error::name, so it is not a reproduction of the recorded build and"
  echo "::error::says nothing about the value. Name the toolchain in $golden,"
  echo "::error::with the measured-on lines, before reading a row off it."
  exit 1
fi

if [ "$got" = "$want" ]; then
  echo "interp_instructions: $got, on the row"
  exit 0
fi

echo "::error::interp_instructions counted $got against $want in $golden,"
echo "::error::a move of $((got - want)). Exactly one of two things is true,"
echo "::error::and they are settled differently."
echo "::error::"
echo "::error::(1) THE CHANGE UNDER TEST MOVED IT. Ordinary ratchet: regenerate"
echo "::error::    $golden, and say in design/compiler-log.md which way it went"
echo "::error::    and why. This row moves for the loader, the prelude and the"
echo "::error::    backend, and it is the ONLY row that moves for codegen at"
echo "::error::    all -- the three compile rows run kanso check, which stops"
echo "::error::    before it. A change to emit that leaves the emitted IR"
echo "::error::    identical shows up here and nowhere else."
echo "::error::"
echo "::error::(2) THE SAME BUILD COUNTED TWO NUMBERS. That is a REPRODUCTION"
echo "::error::    FAILURE. It halts this vein and is hunted to its source --"
echo "::error::    never pinned as a second value, and never recorded as a mode."
echo "::error::    compile_instructions.sh carries the last one and its answer."
echo "::error::"
echo "::error::The interp_binary sha256 and interp_sample lines above are where"
echo "::error::the hunt starts: one sha counting two rows is (2); two shas is"
echo "::error::(1) until the pair is built and both are read."
exit 1
