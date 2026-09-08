#!/bin/sh
# What compiling a LIBRARY FILE costs, counted rather than timed.
#
# `kanso check` routes a single file by its content, and the three routes are
# three different compiles. bench/compile_corpus is a directory, so it is a
# module and goes through compile_module_inner: that is
# scripts/gates/compile_instructions.sh. bench/entry_corpus/main.kso has bare
# statements, so it is an entry and goes through compile_parsed_entry: that is
# scripts/gates/entry_instructions.sh. A file of definitions alone is a
# LIBRARY and goes through compile_library, and until this gate existed
# nothing in the tree counted it.
#
# NOT TO BE CONFUSED WITH scripts/gates/compile_libraries.sh, whose name is one
# letter away and whose subject is unrelated: that one diffs the list of shared
# objects the compiler links against. This one counts instructions.
#
# WHY THE THIRD PATH IS WORTH A ROW. `kanso test` takes it on every run, and so
# does `kanso check` on any single file that is not an entry — which is most
# files in the tree. compile_library merges the dependency program and runs its
# own whole-program check, the same shape the entry path has, and the two paths
# then diverge: kanso#1328 moved a pass in front of that check on the module
# path and kanso#1335 did it on the entry path, and this one still runs in the
# old order. A path with no row is a path whose changes arrive unpriced, which
# is the gap entry_instructions.sh was opened for one path over.
#
# EVERYTHING THIS SHARES WITH THE MODULE ROW IS EXPLAINED THERE, at length and
# with the measurements behind it: why the environment is emptied, why the
# glibc tunables are pinned, why the count is anchored at `kanso::main` rather
# than at the process, why there is no ASLR knob, why the binary's sha and
# section sizes are printed on every run, and why one row holds exactly one
# value. Read compile_instructions.sh; none of it is restated here.
set -e
golden=bench/library_instructions_golden.txt
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

sh scripts/gates/dispatch.sh name

sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir

printf 'library_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|data|bss)[ \t]/ { printf "library_binary %s=%s\n", $1, $2 }'

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
# THE CORPUS IS NAMED RELATIVE TO THE BOX, for the reason
# entry_instructions.sh gives: the count tracks the length of the path the
# compiler is handed, about 160 instructions a character. The directory is
# named library_corpus so the path is the same length as compile_corpus's,
# which keeps the three rows comparable in the one term that is not the
# compiler.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.library ./kanso check \
    library_corpus/library_corpus.kso >/dev/null 2>/dev/null
)

if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi

echo "=== the profile's top frames, inclusive"
callgrind_annotate --inclusive=yes --threshold=99 /tmp/cg.library 2>&1 | head -30

own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.library 2>/dev/null \
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

printf 'library_instructions=%s\n' "$own" > library_ir_got.txt
printf 'library_sample cpu="%s" sha=%.12s row=%s\n' \
  "$(sh scripts/gates/dispatch.sh name | sed -n 's/^silicon: //p')" \
  "$(sha256sum "$box/kanso" | cut -d' ' -f1)" \
  "$(sed -n 's/^library_instructions=//p' library_ir_got.txt)"

echo "=== where the library compile's work is"
callgrind_annotate --threshold=90 /tmp/cg.library 2>&1 | head -40

want=$(sed -n 's/^library_instructions=//p' "$golden")
got=$(sed -n 's/^library_instructions=//p' library_ir_got.txt)
case "$want" in
  '' | *[!0-9]*)
    echo "::error::$golden carries no single library_instructions= value, so"
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
  echo "library_instructions: $got, on the row"
  exit 0
fi

echo "::error::library_instructions counted $got against $want in $golden,"
echo "::error::a move of $((got - want)). Exactly one of two things is true,"
echo "::error::and they are settled differently."
echo "::error::"
echo "::error::(1) THE CHANGE UNDER TEST MOVED IT. Ordinary ratchet: regenerate"
echo "::error::    $golden, and say in design/compiler-log.md which way it went"
echo "::error::    and why. This row moves with the other two compile rows for"
echo "::error::    anything in the lexer, the parser, inference or the shared"
echo "::error::    checks, and alone for anything in compile_library or the"
echo "::error::    order its passes run in."
echo "::error::"
echo "::error::(2) THE SAME BUILD COUNTED TWO NUMBERS. That is a REPRODUCTION"
echo "::error::    FAILURE. It halts this vein and is hunted to its source --"
echo "::error::    never pinned as a second value, and never recorded as a mode."
echo "::error::    compile_instructions.sh carries the last one and its answer."
echo "::error::"
echo "::error::The library_binary sha256 and library_sample lines above are"
echo "::error::where the hunt starts: one sha counting two rows is (2); two"
echo "::error::shas is (1) until the pair is built and both are read."
exit 1
