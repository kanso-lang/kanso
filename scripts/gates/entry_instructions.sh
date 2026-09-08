#!/bin/sh
# What compiling an ENTRY costs, counted rather than timed.
#
# scripts/gates/compile_instructions.sh counts `kanso check compile_corpus`,
# which is a directory, and a directory is a module: it goes through
# compile_module_inner. A file with a top-level expression is an entry and
# goes through compile_parsed_entry, which merges the imports itself and runs
# its own whole-program check at src/lib.rs:160. That call was watched by
# nothing. Every `kanso run` takes it.
#
# The gap was not theoretical. kanso#1324, #1325 and #1326 all changed the
# entry path and all three were measured on a corpus staged in a session's
# temporary directory; the number could not be reproduced by CI, and one of
# them projected a rise of 629 where CI read a fall of 367. A further two
# findings sat unlanded for want of a row that could see them.
#
# EVERYTHING THIS SHARES WITH THE MODULE ROW IS EXPLAINED THERE, at length and
# with the measurements behind it: why the environment is emptied, why the
# glibc tunables are pinned, why the count is anchored at `kanso::main` rather
# than at the process, why there is no ASLR knob, why the binary's sha and
# section sizes are printed on every run, and why one row holds exactly one
# value. Read compile_instructions.sh; none of it is restated here.
set -e
golden=bench/entry_instructions_golden.txt
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

sh scripts/gates/dispatch.sh name

sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir

printf 'entry_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|data|bss)[ \t]/ { printf "entry_binary %s=%s\n", $1, $2 }'

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
# the absolute path is copied and walked — so `entry_corpus/main.kso` and
# `/tmp/kanso-compile-ir/entry_corpus/main.kso` are different numbers for the
# same compile. library_box.sh carries the measurement.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.entry ./kanso check entry_corpus/main.kso \
    >/dev/null 2>/dev/null
)

if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi

echo "=== the profile's top frames, inclusive"
callgrind_annotate --inclusive=yes --threshold=99 /tmp/cg.entry 2>&1 | head -30

own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.entry 2>/dev/null \
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

printf 'entry_instructions=%s\n' "$own" > entry_ir_got.txt
printf 'entry_sample cpu="%s" sha=%.12s row=%s\n' \
  "$(sh scripts/gates/dispatch.sh name | sed -n 's/^silicon: //p')" \
  "$(sha256sum "$box/kanso" | cut -d' ' -f1)" \
  "$(sed -n 's/^entry_instructions=//p' entry_ir_got.txt)"

echo "=== where the entry compile's work is"
callgrind_annotate --threshold=90 /tmp/cg.entry 2>&1 | head -40

want=$(sed -n 's/^entry_instructions=//p' "$golden")
got=$(sed -n 's/^entry_instructions=//p' entry_ir_got.txt)
case "$want" in
  '' | *[!0-9]*)
    echo "::error::$golden carries no single entry_instructions= value, so"
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
  echo "entry_instructions: $got, on the row"
  exit 0
fi

echo "::error::entry_instructions counted $got against $want in $golden,"
echo "::error::a move of $((got - want)). Exactly one of two things is true,"
echo "::error::and they are settled differently."
echo "::error::"
echo "::error::(1) THE CHANGE UNDER TEST MOVED IT. Ordinary ratchet: regenerate"
echo "::error::    $golden, and say in design/compiler-log.md which way it went"
echo "::error::    and why. This row and compile_instructions move together for"
echo "::error::    most front-end changes and independently for anything that"
echo "::error::    touches compile_parsed_entry, load_dependencies or the"
echo "::error::    passes the entry runs and a module does not."
echo "::error::"
echo "::error::(2) THE SAME BUILD COUNTED TWO NUMBERS. That is a REPRODUCTION"
echo "::error::    FAILURE. It halts this vein and is hunted to its source --"
echo "::error::    never pinned as a second value, and never recorded as a mode."
echo "::error::    compile_instructions.sh carries the last one and its answer."
echo "::error::"
echo "::error::The entry_binary sha256 and entry_sample lines above are where"
echo "::error::the hunt starts: one sha counting two rows is (2); two shas is"
echo "::error::(1) until the pair is built and both are read."
exit 1
