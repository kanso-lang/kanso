#!/bin/sh
# What the COMPILER spends emitting, anchored at the frame that does it.
#
#   sh scripts/gates/emit_instructions.sh
#
# WHY IT EXISTS. The three compile rows run `kanso check`, which stops before
# codegen, and the two codegen rows count the child tree — the clang driver,
# `clang -cc1` and `ld` — with kanso's own process excluded because it waits
# for a child and the loop that waits is the scheduler's to size. Between them
# is the work this project actually wrote: turning a checked program into LLVM
# IR. Nothing counted it.
#
# `codegen::emit_ir` is that work and the whole of it, and the anchor is exact
# rather than approximate: measured 2026-09-17 on this project's container,
# four profiles — two readings of the shipped binary and two of a probe binary
# whose `pid_tag_of` returns a constant — all read the frame at 394,910,642
# inclusive, byte for byte, while the process around it moved 233 and then 112.
# It is 92.78% of what kanso's own process spends, so the exclusion drops
# almost nothing else, and what it does drop is the wait.
#
# NO `--trace-children`. The children are the codegen rows' subject and they
# are counted there; here the parent is the whole question, and the wait for
# them sits outside the anchor by construction.
#
# EVERYTHING THIS SHARES WITH THE MODULE ROW IS EXPLAINED THERE, at length and
# with the measurements behind it: why the environment is emptied, why the
# glibc tunables are pinned, why one row holds exactly one value, and why a
# missing frame is a refusal rather than a zero. Read compile_instructions.sh.
set -e
golden=bench/emit_instructions_golden.txt
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

sh scripts/gates/dispatch.sh name

if ! command -v callgrind_annotate >/dev/null; then
  echo "::error::callgrind_annotate is not installed, and the row is read out"
  echo "::error::of its inclusive profile rather than the summary line."
  exit 1
fi

# The SAME staged box the codegen rows use. `codegen_box.sh` stages one place
# by name, the job's steps are sequential, and two boxes built from one tree
# would be two chances to differ for no gain.
box=/tmp/kanso-codegen
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

# THE BOX IS RE-STAGED AND WARMED BEFORE EACH MEASURED RUN, for the reason
# codegen_instructions.sh gives at length: `kanso build X` writes its output
# beside X, so a second run of the same command against the box the first one
# left is a different question; and `cached_runtime_object` keys its cache by
# `TMPDIR`, so a warm-up under a different environment warms a different cache
# and the first MEASURED run pays for compiling runtime.c.
stage_and_warm() {
  sh scripts/gates/codegen_box.sh
  ( cd "$box" && env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" \
      ./kanso build pkg/codegen_corpus >/dev/null 2>&1 )
}

# The frame's inclusive cost, read out of a profile.
emit_cost() {
  callgrind_annotate --inclusive=yes --threshold=99 "$1" 2>/dev/null \
    | awk '/:kanso::codegen::emit_ir \[/ && !seen { gsub(/,/, "", $1); print $1; seen = 1 }'
}

reading() {
  out=$1
  stage_and_warm
  rm -f "$out"
  # `valgrind --tool=callgrind` stays on ONE line. What declares a gate
  # host-bound is `host_gate.sh`, and what makes it eligible is running
  # callgrind -- and `a_host_bound_gate_is_reported_not_credited` reads for
  # both words on a single operative line. Wrapped across a continuation it
  # found the host_gate call and not the callgrind one, and said so.
  ( cd "$box" && env -i PATH=/usr/bin:/bin GLIBC_TUNABLES="$tune" \
      valgrind --tool=callgrind --callgrind-out-file="$out" --cache-sim=no --branch-sim=no \
      ./kanso build pkg/codegen_corpus >/dev/null 2>/dev/null )
}

reading /tmp/cg.emit
printf 'emit_binary sha256=%s\n' "$(sha256sum "$box/kanso" | cut -d' ' -f1)"
echo "=== the profile's top frames, inclusive"
callgrind_annotate --inclusive=yes --threshold=99 /tmp/cg.emit 2>&1 | head -24

got=$(emit_cost /tmp/cg.emit)
case "$got" in
  '' | *[!0-9]*)
    echo "::error::the profile holds no kanso::codegen::emit_ir frame, so this"
    echo "::error::row has nothing to read. The frames the profile DOES hold are"
    echo "::error::printed above; a build that refused would show a short one."
    exit 1
    ;;
esac
printf 'emit_instructions=%s\n' "$got" > emit_got.txt
cat emit_got.txt

want=$(sed -n 's/^emit_instructions=//p' "$golden")
case "$want" in
  "")
    echo "::error::$golden holds no emit_instructions line, so there is nothing"
    echo "::error::to compare the sitting above against. This vein is read by an"
    echo "::error::exact compare and holds exactly one value."
    exit 1
    ;;
esac

if [ "$host" -eq 3 ]; then
  echo "::error::the sitting above was counted on a toolchain $golden does not"
  echo "::error::name, so it is not a reproduction of the recorded build and"
  echo "::error::says nothing about the value."
  exit 1
fi

if [ "$got" = "$want" ]; then
  echo "emit_instructions: $got, on the row"
  exit 0
fi

# THE SAME BUILD, COUNTED AGAIN, BEFORE ANYTHING IS CONCLUDED, and the verdict
# printed before the explanation because GitHub keeps fifty annotations a run.
reading /tmp/cg.emit2
again=$(emit_cost /tmp/cg.emit2)
case "$again" in '' | *[!0-9]*) again=0 ;; esac
printf 'emit_again=%s\n' "$again" >> emit_got.txt
echo "::notice::emit_again=${again} first_reading=${got}"

echo "::error::emit_instructions counted $got against $want in $golden,"
echo "::error::a move of $((got - want))."
if [ "$again" = "$got" ]; then
  echo "::error::VERDICT (1): this binary is stable -- a second count in this"
  echo "::error::same job read $again, the same number. The move is the tree's."
else
  echo "::error::VERDICT (2): REPRODUCTION FAILURE -- this binary counted $got"
  echo "::error::and then $again in one job. This vein is halted."
fi
exit 1
