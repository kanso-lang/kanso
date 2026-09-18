#!/bin/sh
# What CODEGEN costs, at the tier this gate is given, counted rather than timed.
#
#   sh scripts/gates/codegen_instructions.sh dev
#   sh scripts/gates/codegen_instructions.sh release
#
# The three compile gates run `kanso check`, which stops before codegen, and
# startup_instructions.sh runs `kanso play` on one `print`, which is almost all
# loader and emitter. This one runs `kanso build` and counts the WHOLE process
# tree: kanso emitting the IR, the clang driver, `clang -cc1` compiling it, and
# ld linking it. That is what a developer waits for, and it is the only row
# here that counts a program the project did not write.
#
# WHY IT EXISTS. Clay ruled on 2026-09-16 that the objective becomes a
# development welfare, a production welfare and a meta over them, and that the
# two clang tiers land on opposite sides of it: `-O0` is paid on every build in
# the edit-test loop and `-O3 -flto` is paid once for the binary that ships.
# "Under two welfares there is nothing to pick: dev-tier codegen is a
# development term, release-tier codegen is a production term, both counted
# where their cost is paid."
#
# THE CHILDREN ARE WHAT MAKE THIS COUNTABLE. Measured 2026-09-16 with
# `--trace-children` on two runs of one `kanso build`: `clang -cc1` counted
# 532,991,920 both times, the clang driver 31,648,129 both times and ld
# 87,120,997 both times, byte for byte. Only kanso's own process moved, and
# that was the allocator picking a random base address, which kanso#1466 took
# away.
#
# THE GOLDEN NAMES CLANG as well as glibc and rustc, because cc1 and ld are in
# the row and a different clang counts a different number. host_gate.sh reads
# the measured-on line; a host it does not name measures, prints and refuses to
# compare.
#
# BOTH TIERS' CACHES ARE WARMED BEFORE EITHER IS COUNTED.
# `cached_runtime_object` compiles runtime.c once per (profile, runtime hash)
# and writes it to the temp directory, so a row measured on a cold cache counts
# a compile the next row does not. kanso#1461 found that the hard way on the
# start-up row: one binary, one job, 6,018,427 and then 4,869,632, a fifth of
# the row apart.
#
# EVERYTHING THIS SHARES WITH THE MODULE ROW IS EXPLAINED THERE, at length and
# with the measurements behind it: why the environment is emptied, why the
# glibc tunables are pinned, why there is no ASLR knob, why the binary's sha
# and section sizes are printed on every run, and why one row holds exactly one
# value. Read compile_instructions.sh; none of it is restated here.
set -e
# NAMED WITHOUT A TIER, this runs both, because the sweep that has to name it
# runs `sh scripts/gates/<gate>.sh` with no argument and a gate it cannot call
# is a gate nobody sweeps. Both rows are reported before either failure is,
# so one job says where both tiers stand.
if [ $# -eq 0 ]; then
  fail=0
  sh "$0" dev || fail=1
  sh "$0" release || fail=1
  exit "$fail"
fi
tier=$1
case "$tier" in
  dev)     flag="" ;;
  release) flag="--release" ;;
  *) echo "::error::unknown tier $tier; the row is dev or release" >&2; exit 2 ;;
esac
golden=bench/codegen_instructions_${tier}_golden.txt
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

sh scripts/gates/dispatch.sh name

# WHICH PROCESSES A READING SAW, by name. A count says a process is missing; it
# does not say which, and on 2026-09-17 that cost a round: the count read 6 then
# 5, the cause was guessed as an incremental build, the box was re-staged, and
# the next sitting read 6 then 5 again. Every callgrind profile carries a `cmd:`
# line naming the command it counted, so the answer was in the files the whole
# time. This prints the leading word of each, which separates `./kanso`, the two
# `clang` invocations and `ld` without printing the temp paths that differ per
# pid.
#
# AND EACH NAME CARRIES ITS OWN COST, because a row that moves between two
# readings moves inside ONE of these processes. On 2026-09-17 the release
# tier read 7,239,553,333 and then 7,239,550,228 in one job -- 3,105 apart --
# while the dev tier came back byte-identical across the same pair of runs.
# Five processes moved by 3,105 between them and the job log named none of
# them, so the next question had nothing to start from. `name=cost` costs one
# grep per profile and answers it.
# WHERE TWO READINGS DISAGREE, not just by how much. A reproduction failure
# that reports only a magnitude sends the next session looking for a process
# and then a frame, which is two CI rounds to learn what the profiles already
# on disk can say. This pairs the two readings and, for a pair that disagrees,
# diffs the per-function self costs and prints the frames that moved.
# kanso#1469 did this for the compile rows' thirteen and the frame it named
# (`memrchr`, under the line the gate prints) was the whole answer.
#
# TWO THINGS THIS GOT WRONG FIRST, both found by running it against two real
# readings rather than by reading it:
#
#   The key cannot be the program name. A build runs clang three times, so
#   `/usr/bin/clang` paired the probe compile against the driver and reported
#   a 532,767 "disagreement" between two processes that were never the same
#   process. It cannot be the whole argv either: the argv carries temp paths
#   that differ between runs by construction -- the probe's pid, clang's
#   random object suffix. So the key is the argv with those runs of digits and
#   hex flattened, which is stable across runs and still distinguishes the
#   three clangs.
#
#   The percentage column is not one field. callgrind_annotate right-aligns
#   it, so `(100.0%)` is one field and `( 4.02%)` is two, and an awk that
#   counts fields reads the frame name off a different column depending on
#   the size of the number. Every frame name came out blank. The parse is a
#   regex on the whole line now.
#
# It is diagnosis only: it prints and returns, and the caller still fails.
key_of() {
  sed -n 's/^cmd: *//p' "$1" | head -1 \
    | sed -e 's/[0-9a-f]\{6,\}/X/g' -e 's/[0-9]\{4,\}/X/g'
}

self_costs() {
  callgrind_annotate --threshold=100 "$1" 2>/dev/null \
    | sed -n 's/^ *\([0-9,]*\) *([ ]*[0-9.]*%) *\(.*\)$/\1\t\2/p' \
    | sed 's/,//g'
}

why_they_disagree() {
  echo "::error::WHERE THEY DISAGREE, per process and then per frame:"
  for a in $1; do
    [ -f "$a" ] || continue
    ka=$(key_of "$a")
    sum_a=$(grep -o '^summary: [0-9]*' "$a" | tr -dc 0-9)
    for b in $2; do
      [ -f "$b" ] || continue
      [ "$ka" = "$(key_of "$b")" ] || continue
      sum_b=$(grep -o '^summary: [0-9]*' "$b" | tr -dc 0-9)
      [ "$sum_a" = "$sum_b" ] && continue
      echo "::error::  $ka"
      echo "::error::    $sum_a then $sum_b, $((sum_b - sum_a))"
      self_costs "$a" > /tmp/cgfn.first.txt
      self_costs "$b" > /tmp/cgfn.again.txt
      awk -F'\t' 'NR==FNR { was[$2] = $1; seen[$2] = 1; next }
                  { if (seen[$2] && was[$2] != $1)
                      printf "::error::    %+d  %s -> %s  %s\n",
                             $1 - was[$2], was[$2], $1, $2
                    else if (!seen[$2])
                      printf "::error::    ONLY IN THE SECOND READING  %s  %s\n",
                             $1, $2 }' \
        /tmp/cgfn.first.txt /tmp/cgfn.again.txt | head -25
      echo "::error::    (self cost; an empty list means the move is below"
      echo "::error::    what callgrind_annotate resolves at this threshold)"
    done
  done
}

processes_in() {
  for f in "$@"; do
    [ -f "$f" ] || continue
    cost=$(grep -o '^summary: [0-9]*' "$f" | tr -dc 0-9)
    sed -n 's/^cmd: *//p' "$f" | head -1 | awk -v cost="$cost" '{
      n = split($1, p, "/"); name = p[n]
      tag = ""
      if ($0 ~ /kanso_pn_probe/) tag = ":probe"
      else if ($0 ~ /runtime[^ ]*\.c/) tag = ":runtime.c"
      printf "%s%s=%s ", name, tag, cost
    }'
  done
}

# THE ROW IS THE CHILD TREE, AND KANSO'S OWN PROCESS IS EXCLUDED.
# Measured 2026-09-17 on this project's container, two readings of one binary
# in one staging: the three `clang` processes and `ld` came back byte for byte
# in both, and kanso's own process moved 233. Inside it, two frames of 1,346
# differed -- `kanso::build` +189 and `__memcmp_avx2_movbe` +44 -- and a probe
# binary whose `pid_tag_of` returns a constant took the memcmp to zero and the
# total to 112. What is left is one frame: `build` itself, self cost, every
# callee identical, which is the inlined loop that waits for clang. The dev
# tier is the control -- the same code waiting on a child that finishes seven
# times sooner -- and it reproduces exactly.
#
# A loop whose iteration count belongs to the scheduler is external state, and
# the 2026-09-15 rule says a term that cannot be normalized is excluded and the
# exclusion named in the golden's header. So it is: the row counts what codegen
# costs in the processes that do codegen, and the compiler's own emitting gets
# an anchored row of its own rather than a share of a number that wobbles.
#
# The process COUNT still includes kanso, because a build that did not run it
# is not a build and the short-tree guard below is what says so.
is_kanso() {
  # The FIRST WORD and nothing else. A `cmd:` line carries the whole command,
  # and kanso's name turns up inside other processes' arguments -- the
  # convention probe's clang compiles `/tmp/kanso_pn_probe_NNNNNNN.ll`, and
  # the corpus lives under a directory this project named. Matching anywhere
  # in the line calls that clang the compiler and takes the largest
  # deterministic process out of the row.
  case "$(sed -n 's/^cmd: *//p' "$1" | head -1 | awk '{print $1}')" in
    kanso|*/kanso) return 0 ;;
    *) return 1 ;;
  esac
}

box=/tmp/kanso-codegen

# THE BOX IS RE-STAGED BEFORE EACH MEASURED RUN, and that is the whole of what
# kanso#1470 found. `kanso build X` writes its output beside it as `X`, so the
# first measured run leaves the box holding what it just built and the second
# run of the same command is a different question asked of a different box: on
# 2026-09-16 the second reading counted FIVE processes where the first counted
# six, and the dev row fell from 9,273,832,919 to 1,071,604,124 -- a build that
# skipped the work, printed as a REPRODUCTION FAILURE about the compiler. The
# rule it broke is the project's: external state is normalized before it is
# measured. So both readings start from a box `codegen_box.sh` has just
# rebuilt, with both tiers warmed in the same order, and a disagreement after
# that is the compiler's.
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

# KANSO_LTO_JOBS=1 IS PART OF THE SAME NORMALIZATION, and it is the one that
# reaches the linker rather than the allocator. `ld` splits LTO codegen across
# threads; callgrind counts every thread; how the work lands is the scheduler's
# to decide and not the input's. That is why this row drew two faces after
# kanso#1487 had already excluded kanso's own process and certified the child
# tree deterministic: the remaining drift was inside `ld` and nowhere else.
#
# Measured on this container, two links of byte-identical bitcode, both `clang`
# children byte for byte every time:
#
#   different output path, plugin picks:  20,565,047,254  20,565,047,243   -11
#   same output path, plugin picks:       20,565,047,241  20,565,049,584 +2,343
#   same output path, jobs=1:             20,574,502,681  20,574,502,681      0
#
# The magnitude is not a fixed term -- eleven one pair and 2,343 the next --
# which is what ruled out the output path and the pid before it. Only the
# thread count explains a delta that changes size between pairs.
#
# The variable is read by `release_clang` in src/main.rs and is UNSET for every
# build but this measurement, so a user's release build keeps its cores.

# AND THE WARM-UP RUNS UNDER THE MEASUREMENT'S OWN ENVIRONMENT. Re-staging the
# box alone did not settle it: kanso#1470's next sitting still read
# `again_procs=5 first_procs=6`, with the dev row 9,273,832,677 and then
# 1,071,605,357. The profiles name the missing process. Each one carries a
# `cmd:` line, and the extra process in the first reading compiles runtime.c --
# `cached_runtime_object` writes that object to `std::env::temp_dir()`, keyed by
# profile and runtime hash, and keeps it across processes. The box is staged
# fresh; that cache is not in the box.
#
# The warm-ups were meant to fill it and could not, because they ran under the
# job's environment while the measurement runs under `env -i`, and `temp_dir()`
# reads TMPDIR. Two directories, two caches: the first MEASURED run paid for
# runtime.c and the second found it. So the warm-up now runs the identical
# command in the identical environment, which is what the 2026-09-15 rule asks
# for and what "re-stage the box" only half did.
# AND THE OUTPUT PATH IS CLEARED BEFORE EVERY BUILD, WARM OR COUNTED. `ld`
# reads whatever is already at `-o`, and what it finds there changes the row.
# Measured on this gate's own corpus, one binary, one machine, the same `ld`
# command each time, varying only the state of the output path:
#
#     output path absent          5,163,341,031   (twice, to the instruction)
#     output path an empty file   5,163,341,036   (twice, +5)
#     output path 100 bytes       5,163,343,385
#     output path 5 MB            5,163,343,385
#     output path the real binary 5,163,343,385   (+2,354 over absent)
#
# Three groups, each internally identical to the instruction, and size does
# not matter once the file is non-empty. So it is the prior CONTENTS of the
# output path, not the name, not the pid and not the run: three different
# object-file names with the output in the same state all read identically.
#
# The gate was reading the third group by accident. `stage_and_warm` wipes the
# box and then warms both tiers, so the counted build always found the warm-up's
# binary sitting at `-o` -- correct by luck, and a reordering or a dropped
# warm-up would have moved the row by 2,354 with nothing to say why. Clearing
# is the 2026-09-15 rule applied literally: put it in a known initial state
# every single run. Absent is the state a fresh box gives and the only one of
# the three that needs nothing created to reach it.
clear_output() {
  rm -f "$box/codegen_corpus" "$box/codegen_corpus.ll"
}

stage_and_warm() {
  sh scripts/gates/codegen_box.sh
  # Warm BOTH tiers, whichever one this run counts, so the row does not depend
  # on which of the two the job happened to ask for first.
  for warm_flag in "" "--release"; do
    clear_output
    ( cd "$box" && env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" KANSO_LTO_JOBS=1 \
        ./kanso build pkg/codegen_corpus $warm_flag >/dev/null 2>&1 )
  done
}
stage_and_warm

printf 'codegen_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|rodata|data|bss)[ \t]/ { printf "codegen_binary %s=%s\n", $1, $2 }'
printf 'codegen_clang %s\n' "$(clang --version | head -1)"


rm -f /tmp/cg.codegen.$tier.*
clear_output
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" KANSO_LTO_JOBS=1 valgrind --tool=callgrind \
    --trace-children=yes --callgrind-out-file=/tmp/cg.codegen.$tier.%p \
    ./kanso build pkg/codegen_corpus $flag >/dev/null 2>/dev/null
)

# The row is the sum over the tree, read from each profile's own summary line.
# There is no anchor here the way there is on the compile rows: the subject IS
# the whole tree, clang and ld included, and kanso's own process is the smaller
# half of it at the release tier.
sum=0
seen=0
for f in /tmp/cg.codegen.$tier.*; do
  [ -f "$f" ] || continue
  n=$(grep -o '^summary: [0-9]*' "$f" | tr -dc 0-9)
  [ -n "$n" ] || continue
  seen=$((seen + 1))
  if is_kanso "$f"; then
    echo "::notice::codegen_${tier}_kanso_excluded=${n}"
    continue
  fi
  sum=$((sum + n))
done
printf 'codegen_processes %s=%s\n' "$tier" "$seen"
echo "::notice::codegen_procs_${tier} first=[$(processes_in /tmp/cg.codegen.$tier.*)]"
printf 'codegen_instructions_%s=%s\n' "$tier" "$sum" > codegen_${tier}_got.txt
cat codegen_${tier}_got.txt

if [ "$seen" -lt 4 ]; then
  echo "::error::this build ran $seen processes and a real one runs five:"
  echo "::error::kanso, the clang driver, clang -cc1, ld, and the linker's own"
  echo "::error::child. A build that is REFUSED still emits the IR first, so a"
  echo "::error::short tree is the shape of a build that never wrote anything."
  exit 1
fi

want=$(sed -n "s/^codegen_instructions_${tier}=//p" "$golden")
got=$(sed -n "s/^codegen_instructions_${tier}=//p" codegen_${tier}_got.txt)

case "$want" in
  "")
    echo "::error::$golden holds no codegen_instructions_${tier} line, so"
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
  echo "codegen_instructions_${tier}: $got, on the row"
  exit 0
fi

# THE SAME BUILD, COUNTED AGAIN, BEFORE ANYTHING IS CONCLUDED. The compile
# rows take this second reading for the reason kanso#1463 gives at length: a
# number that disagrees says how much and nothing about where, and the two
# cases below are settled differently.
stage_and_warm
rm -f /tmp/cg.codegen.${tier}b.*
clear_output
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" KANSO_LTO_JOBS=1 valgrind --tool=callgrind \
    --trace-children=yes --callgrind-out-file=/tmp/cg.codegen.${tier}b.%p \
    ./kanso build pkg/codegen_corpus $flag >/dev/null 2>/dev/null
)
again=0
again_seen=0
for f in /tmp/cg.codegen.${tier}b.*; do
  [ -f "$f" ] || continue
  n=$(grep -o '^summary: [0-9]*' "$f" | tr -dc 0-9)
  [ -n "$n" ] || continue
  again_seen=$((again_seen + 1))
  if is_kanso "$f"; then
    echo "::notice::codegen_${tier}_kanso_excluded_again=${n}"
    continue
  fi
  again=$((again + n))
done
# HOW MANY PROCESSES EACH READING SAW, because a second reading that counts
# FEWER of them is not measuring the same thing and its disagreement says
# nothing about the compiler. A build here is five processes -- kanso, clang
# at two tiers, and ld -- and `codegen_box.sh` records that an early attempt
# at this row read two where a real build is five. The first reading already
# counts them into `seen`; the second did not, so a drop was invisible and
# read as a reproduction failure.
printf 'codegen_%s_again row=%s (the first reading was %s)\n' "$tier" "$again" "$got"
# As a notice too, so it survives as an annotation: plain stdout reaches only
# the job log, which is the one place a reader may not be able to fetch.
echo "::notice::codegen_${tier}_again=${again} first_reading=${got} again_procs=${again_seen} first_procs=${seen}"
echo "::notice::codegen_procs_${tier} again=[$(processes_in /tmp/cg.codegen.${tier}b.*)]"
printf 'codegen_%s_again=%s\n' "$tier" "$again" >> codegen_${tier}_got.txt

echo "::error::codegen_instructions_${tier} counted $got against $want in $golden,"
echo "::error::a move of $((got - want)). Exactly one of two things is true,"
echo "::error::and they are settled differently."
echo "::error::"
# THE VERDICT GOES FIRST, and that is not style. GitHub keeps at most fifty
# annotations per check run, and this gate emits a dozen lines of explanation
# per failing row. On kanso#1470 both codegen rows and all three compile rows
# failed in one job; the two lines below, which say WHICH of the two cases it
# is, fell off the end of the cap and the job could only be read as far as
# "a move of -6531790". The explanation is worth having and it is worth
# nothing ahead of the answer.
if [ "$again" = "$got" ]; then
  echo "::error::VERDICT (1): this binary is stable -- a second count in this"
  echo "::error::same job read $again, the same number."
else
  echo "::error::VERDICT (2): REPRODUCTION FAILURE -- this binary counted $got"
  echo "::error::and then $again in one job. This vein is halted."
fi
echo "::error::"
echo "::error::(1) THE CHANGE UNDER TEST MOVED IT. Ordinary ratchet: regenerate"
echo "::error::    $golden, and say in design/compiler-log.md which way it went"
echo "::error::    and why. This row is the ONLY one that counts codegen at"
echo "::error::    all -- the three compile rows run kanso check, which stops"
echo "::error::    before it -- so a change to the emitter that leaves the"
echo "::error::    emitted IR identical shows up here and nowhere else. It also"
echo "::error::    moves for the IR the emitter writes, since clang then reads"
echo "::error::    a different program."
echo "::error::"
echo "::error::(2) THE SAME BUILD COUNTED TWO NUMBERS. That is a REPRODUCTION"
echo "::error::    FAILURE. It halts this vein and is hunted to its source --"
echo "::error::    never pinned as a second value, and never recorded as a mode."
echo "::error::    The last two were the allocator picking a random base"
echo "::error::    address (kanso#1466) and a cold runtime-object cache"
echo "::error::    (kanso#1461); this gate warms the cache before it counts."
echo "::error::"
if [ "$again" = "$got" ]; then
  echo "::error::THIS BINARY IS STABLE. A second count in this same job, on"
  echo "::error::this same binary, read $again -- the same number. So the"
  echo "::error::disagreement is with the GOLDEN and not within the run, and"
  echo "::error::this is (1) unless the golden's own sitting differed in"
  echo "::error::something outside the diff. The codegen_binary sha256 and the"
  echo "::error::codegen_clang line above are what to compare against it."
else
  why_they_disagree "/tmp/cg.codegen.$tier.*" "/tmp/cg.codegen.${tier}b.*"
echo "::error::THIS BINARY COUNTED TWO NUMBERS IN ONE JOB: $got and then"
  echo "::error::$again, on one binary, one corpus and one machine. That is"
  echo "::error::(2), settled here rather than by comparing runs, and it halts"
  echo "::error::this vein."
fi
exit 1
