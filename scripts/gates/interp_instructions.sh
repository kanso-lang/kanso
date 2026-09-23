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
  | awk '/^\.(text|rodata|data|bss)[ \t]/ { printf "interp_binary %s=%s\n", $1, $2 }'

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

# THE PRINTED LINE COMES OFF, the same way and for the same reason it comes off
# the module row. `interp_corpus` prints what it decoded, LineWriter runs
# `core::slice::memchr::memrchr` over the formatted bytes to find the last
# newline, and what that frame costs moves with the binary's LAYOUT rather than
# with anything the interpreter does. kanso#1487 measured it on the module row:
# five CI builds on 2026-09-17, across trees whose compiler source was
# identical, drew two faces THIRTEEN apart on the module, entry and library
# rows, while start-up -- whose gate prints nothing -- gave one value on all
# five. That fix landed on the three `kanso check` rows and not here, and this
# row has been drawing the same two faces ever since: kanso#1486's two sittings
# read 2,182,526,878 and 2,182,526,865, thirteen apart, on trees whose only
# difference was three goldens, a page and a log entry.
#
# So the term is excluded, per the 2026-09-15 rule: what cannot be normalized
# is left out and the exclusion is named in the golden's header. The figure is
# printed as a notice rather than only subtracted, because a number that only
# ever appears subtracted cannot answer the next drift.
#
# IT IS A FUNCTION because the second count below must read the row the same
# way this one does, and for a while it did not. See the note at `again`.
printed_cost() {
  callgrind_annotate --inclusive=yes --threshold=100 "$1" 2>/dev/null \
    | awk '/:std::io::stdio::_print \[/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }'
}
printed=$(printed_cost /tmp/cg.interp)
case "$printed" in '' | *[!0-9]*) printed=0 ;; esac
case "$own" in
  '' | *[!0-9]*) ;;
  *) own=$((own - printed)) ;;
esac
echo "::notice::interp_printed=${printed}"
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

sh "$(dirname "$0")/function_table.sh" /tmp/cg.interp interp

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

# THE SAME BINARY, COUNTED AGAIN, BEFORE ANYTHING IS CONCLUDED.
#
# The two cases below are settled differently and the job log could not tell
# them apart: a reader had to compare sha lines across two runs by hand. On
# 2026-09-16 the compile rows did that twice in one evening -- kanso#1459's two
# rounds carried identical compiler source and read 13 apart on all three, and
# the start-up row read 33 apart on the same kind of pair. One more reading
# inside this job separates "this binary counted two numbers" from "two runs
# differed in something outside the diff", for one callgrind pass on a run that
# was going to fail anyway. kanso#1463 does the same for the compile gates.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.interp2 ./kanso run interp_corpus --interp \
    >/dev/null 2>/dev/null
)
again=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.interp2 2>/dev/null \
        | awk '/kanso::run_interpreted_on_stack/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
# THE PRINTED LINE COMES OFF THIS READING TOO. For one day it did not. The
# first reading is the frame MINUS the printed line; this one was the raw
# frame, so a binary that counted the same number twice was reported as having
# counted two numbers exactly `interp_printed` apart -- and the gate said so in
# words that assert the opposite of what happened. kanso#1502 and kanso#1504
# both read 1,138,001,437 and then 1,138,004,452 on 2026-09-18, a gap of 3,015,
# with `interp_printed=3015` in both job logs: two branches, two runners, the
# same pair, and both accused of a reproduction failure they did not have.
#
# WHERE IT CAME FROM. kanso#1487 gave the three compile-side gates the
# subtraction on 2026-09-17 and wrote the reader as `printed_cost()`, so both
# of their readings got it for free. kanso#1505 brought it to this row the same
# day and open-coded the pipeline instead, which reached the first reading and
# not this one. Naming the reader is what makes the second call obvious, and
# the spec below is what makes it checked.
again_printed=$(printed_cost /tmp/cg.interp2)
case "$again_printed" in '' | *[!0-9]*) again_printed=0 ;; esac
again=$((again - again_printed))
printf 'interp_again row=%s (the first reading was %s)\n' "$again" "$got"


# THE SILICON, COMPARED RATHER THAN NAMED. `dispatch.sh name` above prints the
# family and the model, which are the two rows most likely to be equal between
# two different runners. The block in bench/dispatch.txt is the whole feature
# set glibc's ifunc resolvers read, and it is consulted HERE -- only when a row
# has already moved -- because a resolver that picked a different memcpy is one
# of the things "outside the diff" can mean. Seven distinct blocks were seen
# across ninety-odd jobs on 2026-09-17, differing in 57 rows and in the basic
# family itself. It never refuses on its own: `differs` answers 2 when it
# cannot tell, and this reports whichever answer it gives.
silicon=0
sh scripts/gates/dispatch.sh differs || silicon=$?
case "$silicon" in
  0) echo "::error::THE SILICON MATCHES bench/dispatch.txt, so the resolvers"
     echo "::error::glibc picked are the ones that block records." ;;
  2) echo "::error::THE SILICON CANNOT BE COMPARED -- no block recorded, or"
     echo "::error::this loader reports no features." ;;
  *) echo "::error::THE SILICON DIFFERS from bench/dispatch.txt. The rows are"
     echo "::error::printed above. A different resolver is one of the things"
     echo "::error::a move outside the diff can be." ;;
esac
echo "::error::"
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
if [ "$again" = "$got" ]; then
  echo "::error::THIS BINARY IS STABLE. A second count in this same job, on"
  echo "::error::this same binary, read $again -- the same number. So the"
  echo "::error::disagreement is with the GOLDEN and not within the run, and"
  echo "::error::this is (1) unless the golden's own sitting differed in"
  echo "::error::something outside the diff. The interp_binary sha256 and the"
  echo "::error::silicon line above are what to compare against that sitting."
else
  echo "::error::THIS BINARY COUNTED TWO NUMBERS IN ONE JOB: $got and then"
  echo "::error::$again, on one binary, one corpus and one machine. That is"
  echo "::error::(2), settled here rather than by comparing runs, and it halts"
  echo "::error::this vein."
fi
exit 1
