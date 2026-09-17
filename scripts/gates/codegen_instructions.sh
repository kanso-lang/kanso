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
sh scripts/gates/codegen_box.sh
box=/tmp/kanso-codegen

printf 'codegen_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
size --format=sysv "$box/kanso" \
  | awk '/^\.(text|data|bss)[ \t]/ { printf "codegen_binary %s=%s\n", $1, $2 }'
printf 'codegen_clang %s\n' "$(clang --version | head -1)"

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

# Warm BOTH tiers, whichever one this run counts, so the row does not depend on
# which of the two the job happened to ask for first.
( cd "$box" && ./kanso build pkg/codegen_corpus >/dev/null 2>&1 )
( cd "$box" && ./kanso build pkg/codegen_corpus --release >/dev/null 2>&1 )

rm -f /tmp/cg.codegen.$tier.*
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
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
  sum=$((sum + n))
  seen=$((seen + 1))
done
printf 'codegen_processes %s=%s\n' "$tier" "$seen"
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
rm -f /tmp/cg.codegen.${tier}b.*
(
  cd "$box"
  env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" valgrind --tool=callgrind \
    --trace-children=yes --callgrind-out-file=/tmp/cg.codegen.${tier}b.%p \
    ./kanso build pkg/codegen_corpus $flag >/dev/null 2>/dev/null
)
again=0
for f in /tmp/cg.codegen.${tier}b.*; do
  [ -f "$f" ] || continue
  n=$(grep -o '^summary: [0-9]*' "$f" | tr -dc 0-9)
  [ -n "$n" ] && again=$((again + n))
done
printf 'codegen_again_%s row=%s (the first reading was %s)\n' "$tier" "$again" "$got"
# As a notice too, so it survives as an annotation: plain stdout reaches only
# the job log, which is the one place a reader may not be able to fetch.
echo "::notice::codegen_again_${tier}=${again} first_reading=${got}"
printf 'codegen_again_%s=%s\n' "$tier" "$again" >> codegen_${tier}_got.txt

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
  echo "::error::THIS BINARY COUNTED TWO NUMBERS IN ONE JOB: $got and then"
  echo "::error::$again, on one binary, one corpus and one machine. That is"
  echo "::error::(2), settled here rather than by comparing runs, and it halts"
  echo "::error::this vein."
fi
exit 1
