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
  env -i PATH=/usr/bin:/bin KANSO_QUIET=1 GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
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

# THE SAME BINARY, COUNTED AGAIN, BEFORE ANYTHING IS CONCLUDED.
#
# The two cases below are settled differently and the job log could not tell
# them apart. A reader had to compare sha lines across two runs by hand, and on
# 2026-09-16 that comparison came back confounded: kanso#1459's two rounds
# carried identical compiler source -- round two changed goldens, the log, the
# floor and one page and nothing the compiler compiles -- and all three rows
# read exactly 13 higher, with BOTH a different binary sha and a different
# runner CPU model between them (AMD family 0x19 model 0x1 against model 0x11).
# Two variables, one observation, and no way to separate them from outside.
#
# One more reading inside this job separates the halves for nothing. It costs a
# second callgrind pass and only on a run that was going to fail anyway, and it
# answers the question the error text below has always asked a reader to answer
# by hand.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin KANSO_QUIET=1 GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.library2 ./kanso check \
    library_corpus/library_corpus.kso >/dev/null 2>/dev/null
)
again=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.library2 2>/dev/null \
        | awk '/kanso::main/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
printf 'library_again row=%s (the first reading was %s)\n' "$again" "$got"
# AND INTO THE ARTIFACT, because the job log is the expensive place to read it
# from. The `*_got.txt` files are catted in one step at the end of the job,
# eighty lines from its tail, where the callgrind output above this is several
# hundred. A reader who has to fetch the whole job to learn whether the binary
# was stable is a reader who will not bother.
printf 'library_again=%s\n' "$again" >> library_ir_got.txt

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
if [ "$again" = "$got" ]; then
  echo "::error::THIS BINARY IS STABLE. A second count in this same job, on"
  echo "::error::this same binary, read $again -- the same number. So the"
  echo "::error::disagreement is with the GOLDEN and not within the run, and"
  echo "::error::this is (1) unless the golden's own sitting differed in"
  echo "::error::something outside the diff. The library_binary sha256 and the"
  echo "::error::silicon line above are what to compare against that sitting."
else
  echo "::error::THIS BINARY COUNTED TWO NUMBERS IN ONE JOB: $got and then"
  echo "::error::$again, on one binary, one corpus and one machine. That is"
  echo "::error::(2), settled here rather than by comparing runs, and it halts"
  echo "::error::this vein."
  echo "::error::"
  echo "::error::The two profiles are still on disk and the diff below says"
  echo "::error::WHERE they part, function by function. A per-process term"
  echo "::error::names one or two frames near the process's entry; a compiler"
  echo "::error::difference spreads across the passes that ran."
  echo "=== where the two readings part"
  sh scripts/gates/profile_diff.sh /tmp/cg.library /tmp/cg.library2 || true
fi
exit 1
