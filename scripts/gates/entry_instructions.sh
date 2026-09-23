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
# AND AS A NOTICE, because this is the reading that decides the open
# question about this row. Eight runs of ONE binary in one container,
# with this gate's own box, command and tunables, read 36,384,683 every
# time -- so the cross-run thirteen is not two runs of one binary. The
# remaining candidate is two binaries, which is exactly what the 508
# documented in compile_instructions.sh turned out to be, and the sha is
# what tells them apart. It was printed to stdout, and stdout reaches
# only the job log.
echo "::notice::entry_binary sha256=$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|rodata|data|bss)[ \t]/ { printf "entry_binary %s=%s\n", $1, $2 }'
# AND THE SAME THREE AS A NOTICE. kanso#1479 made the sha a notice for a
# reason that applies here with more force: the sections are the only printed
# property that distinguishes two BUILDS of one source, and stdout reaches
# only the job log, whose blob host answers a fetch from here with `CONNECT
# tunnel failed, response 403`. On 2026-09-17 two attempts of run 35197408041
# -- one commit, two builds, two CPU models -- printed different sha256s and
# byte-identical rows, while main's build of a tree that cannot change the
# compiler read thirteen away. That is the shape of a layout difference
# between BUILDS, and the sections are what would say so; they were printed
# on both runs and readable on neither.
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|rodata|data|bss)[ \t]/ { printf "%s=%s ", $1, $2 }' > /tmp/entry.sections
echo "::notice::entry_binary sections $(cat /tmp/entry.sections)"

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
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" LD_PRELOAD="$blind" valgrind --tool=callgrind \
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

# THE RESULT LINE IS NOT COMPILER WORK, AND THE FRAME UNDER IT IS WHERE THE
# THIRTEEN LIVED.
#
# `kanso check` prints one line when it finishes, and `kanso::main` inclusive
# counts it. Under it LineWriter runs `core::slice::memchr::memrchr` over the
# formatted bytes to find the last newline, and that frame's cost moves with
# the binary's layout. Two CI builds of ONE source -- kanso#1477 at 716fcfc4
# and at 8e4e5665, whose commit touched only goldens, the log and a page --
# read 35,965,150 and 35,965,137 on this row. Within a build the reading is
# exact; across builds it drew.
#
# Neither way of not printing helps, because both change the process the gate
# measures. On one box, `kanso check compile_corpus`:
#
#   env -i, two variables, printing      36,817,649
#   env -i, three variables, printing    36,829,255   +11,606
#   env -i, three variables, quiet       36,828,139    -1,116
#   two variables, printing              36,817,388
#   two variables, --quiet               36,818,319      +931
#
# An environment variable costs ten times what the quiet saves (the compiler
# asks getenv about seven thousand times and each ask walks the block), and an
# argv entry costs about twice it. Both move the initial process layout, which
# is the same class of thing the thirteen is.
#
# So the term is EXCLUDED instead, per the 2026-09-15 rule: what cannot be
# normalized is left out and the exclusion is named in the golden's header.
# `std::io::stdio::_print` is reached once per run, from `kanso::driven`, and
# its whole subtree is the line -- 748 instructions on the profile this was
# read from, with `memrchr`'s 133 inside it. Nothing else in a `kanso check`
# prints to stdout; diagnostics go to stderr.
printed_cost() {
  callgrind_annotate --inclusive=yes --threshold=100 "$1" 2>/dev/null \
    | awk '/:std::io::stdio::_print \[/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }'
}
own=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.entry 2>/dev/null \
      | awk '/kanso::main/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
printed=$(printed_cost /tmp/cg.entry)
case "$printed" in '' | *[!0-9]*) printed=0 ;; esac
own=$((own - printed))
# WHAT WAS TAKEN OFF, where a reader can see it. If this row ever drifts
# again, the first question is whether the printed line's own cost moved --
# and that question is unanswerable from a number that only ever appears
# subtracted.
echo "::notice::entry_printed=${printed}"
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

sh "$(dirname "$0")/function_table.sh" /tmp/cg.entry entry

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
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" LD_PRELOAD="$blind" valgrind --tool=callgrind \
    --callgrind-out-file=/tmp/cg.entry2 ./kanso check entry_corpus/main.kso \
    >/dev/null 2>/dev/null
)
again=$(callgrind_annotate --inclusive=yes --threshold=100 /tmp/cg.entry2 2>/dev/null \
        | awk '/kanso::main/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }')
again_printed=$(printed_cost /tmp/cg.entry2)
case "$again_printed" in '' | *[!0-9]*) again_printed=0 ;; esac
again=$((again - again_printed))
printf 'entry_again row=%s (the first reading was %s)\n' "$again" "$got"
# AND INTO THE ARTIFACT, because the job log is the expensive place to read it
# from. The `*_got.txt` files are catted in one step at the end of the job,
# eighty lines from its tail, where the callgrind output above this is several
# hundred. A reader who has to fetch the whole job to learn whether the binary
# was stable is a reader who will not bother.
printf 'entry_again=%s\n' "$again" >> entry_ir_got.txt
# And as a notice, so it survives as an ANNOTATION. The artifact and
# the job log both need fetching; annotations come back over the
# ordinary API, which is the only path a reader is guaranteed.
echo "::notice::entry_again=${again} first_reading=${got}"


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
echo "::error::entry_instructions counted $got against $want in $golden,"
echo "::error::a move of $((got - want)). Exactly one of two things is true,"
echo "::error::and they are settled differently."
echo "::error::"
# THE VERDICT GOES FIRST, and that is not style. GitHub keeps at most
# fifty annotations per check run, and each failing row here emits a
# dozen explanatory lines. When all three compile rows part at once --
# which is what the cross-run thirteen does, every time -- the two
# lines below fell past the cap, and the job could be read as far as
# "a move of -13" and no further. A gate that had already settled the
# question reported nothing. The explanation is worth having and it is
# worth nothing ahead of the answer.
if [ "$again" = "$got" ]; then
  echo "::error::VERDICT (1): this binary is stable -- a second count in"
  echo "::error::this same job read $again, the same number."
else
  echo "::error::VERDICT (2): REPRODUCTION FAILURE -- this binary counted"
  echo "::error::$got and then $again in one job. This vein is halted."
fi
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
if [ "$again" = "$got" ]; then
  echo "::error::THIS BINARY IS STABLE. A second count in this same job, on"
  echo "::error::this same binary, read $again -- the same number. So the"
  echo "::error::disagreement is with the GOLDEN and not within the run, and"
  echo "::error::this is (1) unless the golden's own sitting differed in"
  echo "::error::something outside the diff. The entry_binary sha256 and the"
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
  sh scripts/gates/profile_diff.sh /tmp/cg.entry /tmp/cg.entry2 || true
fi
exit 1
