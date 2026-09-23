#!/bin/sh
# What a RUN costs before it has any work to do, counted rather than timed.
#
# The other three instruction gates count what compiling a body of code costs
# and all three run `kanso check`, which stops before codegen:
# compile_instructions.sh a directory, entry_instructions.sh a file with
# statements, library_instructions.sh a file of definitions. This one runs
# `kanso play` on a program holding a single `print`. What it counts is the
# loader, the prelude and the whole of the backend deciding what to emit for
# almost nothing -- and a one-line program is the point of it, because what is
# left when the program is that small is start-up.
#
# WHY IT EXISTS. Clay ruled on 2026-09-16 that the objective becomes a
# development welfare, a production welfare and a meta over them, and that
# start-up belongs to the development side: `kanso test` pays it on every
# invocation and production never pays it once. The counters for that side did
# not exist and this is the first of them. It is an exact vein of its own and
# not yet an objective term, the way `.text` is under the 2026-09-05 ruling.
#
# Opening it was enough to find something. `codegen::Backend::emit`'s
# `declares` filter re-split DECLARES and re-scanned its non-declare lines on
# each of about 1,204 asks; three quarters of start-up was substring search and
# 92.6% of it had that one owner. The row fell from 69,183,407 to 4,919,980 on
# the container, 14.06x, with the emitted IR byte-identical.
#
# EVERYTHING THIS SHARES WITH THE MODULE ROW IS EXPLAINED THERE, at length and
# with the measurements behind it: why the environment is emptied, why the
# glibc tunables are pinned, why the count is anchored at `kanso::main` rather
# than at the process, why there is no ASLR knob, why the binary's sha and
# section sizes are printed on every run, and why one row holds exactly one
# value. Read compile_instructions.sh; none of it is restated here.
set -e
golden=bench/startup_instructions_golden.txt
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

sh scripts/gates/dispatch.sh name

sh scripts/gates/library_box.sh
box=/tmp/kanso-compile-ir

printf 'startup_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|rodata|data|bss)[ \t]/ { printf "startup_binary %s=%s\n", $1, $2 }'

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
# memcmp and bcmp are replaced for the counted run by one whose cost depends on
# the length and the first difference and not on where the operands sit.
# libc's avx2 memcmp takes a longer branch when either operand lies within 32
# bytes of a page end, so a change that only moves strings in .rodata moved
# this row. address_blind.sh proves the replacement at two page offsets before
# printing its path.
blind=$(sh scripts/gates/address_blind.sh)
# THE CORPUS IS NAMED RELATIVE TO THE BOX. The count tracks the length of the
# path the compiler is handed — about 160 instructions a character, because
# the absolute path is copied and walked — so `entry_corpus/main.kso` and
# `/tmp/kanso-compile-ir/entry_corpus/main.kso` are different numbers for the
# same compile. library_box.sh carries the measurement.
# THE CACHES ARE WARMED BEFORE ANYTHING IS COUNTED, and this is not a nicety.
#
# `kanso play` takes the native path: it lowers the program, compiles
# `runtime.c` into `kanso_runtime_<profile>_<key>.o` in the temp directory and
# links a `kanso_run_<key>` beside it, both keyed on a hash and both reused by
# the next process that wants them. So the FIRST process of a job pays for
# staging and writing them and every one after it does not.
#
# It cost this vein a reproduction failure on 2026-09-17: one binary, one job,
# read 6,018,427 and then 4,869,632 -- 1,148,795 apart, which is a fifth of the
# row. The 2026-09-15 ruling says external state is put into a known state
# before it is measured rather than explained afterwards, and a cache is
# exactly that: this run puts it into the warm one, which is the state every
# reading after the first would have seen anyway.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" LD_PRELOAD="$blind" \
    ./kanso play startup_corpus/main.kso >/dev/null 2>/dev/null
)
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" LD_PRELOAD="$blind" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.startup ./kanso play startup_corpus/main.kso \
    >/dev/null 2>/dev/null
)

if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi

echo "=== the profile's top frames, inclusive"
callgrind_annotate --inclusive=yes --threshold=99 /tmp/cg.startup 2>&1 | head -30

# THIS GATE'S PROGRAM PRINTS NOTHING TO STDOUT through Rust, which is why the
# printed line is not subtracted here the way it is on the four rows beside
# this one. `startup_corpus/main.kso` holds a single `print` and that output
# does reach stdout -- but `kanso play` takes the NATIVE path, so the program's
# print is the C runtime writing directly and never enters
# `std::io::stdio::_print`. No LineWriter, no `memrchr` over the formatted
# bytes, and so no frame whose cost moves with the binary's layout.
#
# The measurement agrees with the mechanism. kanso#1487 read five CI builds on
# 2026-09-17 across trees whose compiler source was identical: the module,
# entry and library rows each drew two faces thirteen apart, and this row gave
# ONE value on all five. That is the row where the exclusion would have nothing
# to take off.
#
# tests/every_anchored_gate_answers_for_the_printed_line.rs is what makes this
# paragraph exist rather than be assumed.
own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.startup 2>/dev/null \
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

printf 'startup_instructions=%s\n' "$own" > startup_ir_got.txt
printf 'startup_sample cpu="%s" sha=%.12s row=%s\n' \
  "$(sh scripts/gates/dispatch.sh name | sed -n 's/^silicon: //p')" \
  "$(sha256sum "$box/kanso" | cut -d' ' -f1)" \
  "$(sed -n 's/^startup_instructions=//p' startup_ir_got.txt)"

echo "=== where the entry compile's work is"
callgrind_annotate --threshold=90 /tmp/cg.startup 2>&1 | head -40

sh "$(dirname "$0")/function_table.sh" /tmp/cg.startup startup

want=$(sed -n 's/^startup_instructions=//p' "$golden")
got=$(sed -n 's/^startup_instructions=//p' startup_ir_got.txt)
case "$want" in
  '' | *[!0-9]*)
    echo "::error::$golden carries no single startup_instructions= value, so"
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
  echo "startup_instructions: $got, on the row"
  exit 0
fi

# THE SAME BINARY, COUNTED AGAIN, BEFORE ANYTHING IS CONCLUDED.
#
# The two cases below are settled differently and the job log could not tell
# them apart: a reader had to compare sha lines across two runs by hand. On
# 2026-09-16 this row did it twice in one evening -- CI read 6,018,394 and then
# 6,018,427 on identical compiler source, a move of 33 -- and the three compile
# rows did the same thing with 13 on kanso#1459. One more reading inside this
# job separates "this binary counted two numbers" from "two runs differed in
# something outside the diff", for the price of one callgrind pass on a run
# that was going to fail anyway.
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" LD_PRELOAD="$blind" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.startup2 ./kanso play startup_corpus/main.kso \
    >/dev/null 2>/dev/null
)
again=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.startup2 2>/dev/null \
        | awk '/kanso::main/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
printf 'startup_again row=%s (the first reading was %s)\n' "$again" "$got"


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
echo "::error::startup_instructions counted $got against $want in $golden,"
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
  echo "::error::something outside the diff. The startup_binary sha256 and the"
  echo "::error::silicon line above are what to compare against that sitting."
else
  echo "::error::THIS BINARY COUNTED TWO NUMBERS IN ONE JOB: $got and then"
  echo "::error::$again, on one binary, one corpus and one machine. That is"
  echo "::error::(2), settled here rather than by comparing runs, and it halts"
  echo "::error::this vein."
fi
exit 1
