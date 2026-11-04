#!/bin/sh
# Every counter gate at once, and the goldens they read.
#
# Each is its own file, so following the rule in CLAUDE.md -- "counters changed,
# regenerate every vein in the same PR" -- meant remembering a filename apiece
# and which golden each one writes. There were nine when this was written; a
# branch that changed how closures are built regenerated the .mem vein, the four
# code goldens and the wasm blob and missed all nine, and CI found them one round
# late, which cost eight minutes for something a container answers in twenty
# seconds. `--list` prints how many there are now; the sweep reads them from
# the gates themselves, so no count is written down here.
#
# It does NOT stop at the first divergence. Each vein is an independent
# dimension of the same run, and the CI job that owns them says why: stopping
# early once kept the instruction counts from being measured at all for two
# runs, and the decoder was 12.2% slower the whole time with nothing able to
# say so. The summary at the end names every vein that moved.
#
#   sh scripts/gates/all_counters.sh           read-only, one summary
#   sh scripts/gates/all_counters.sh --write   regenerate each golden that moved
#   sh scripts/gates/all_counters.sh --list    print each vein and its golden
#
# --write preserves each golden's comment header. Two of them have one --
# cost_golden_wide.txt and cost_golden_pend.txt -- and a plain `cp` of the
# measured file over the golden strips it, which is a trap worth holding here
# rather than remembering.
set -e
write=0
[ "$1" = "--write" ] && write=1


# Each vein is a gate, `scripts/gates/<vein>_counters.sh`, and the sweep takes
# the program and the golden from the gate itself: the COUNTING binary the gate
# runs under KANSO_COUNTERS=1, and the `bench/cost_golden*.txt` it diffs. There
# is no second list to keep in step. One once lived here, and repointing only
# the gates at the `-counters` binaries left this sweep reading the wrong ones
# and calling every vein moved (kanso#2291).
veins=""
for gate in scripts/gates/*_counters.sh; do
  vein=$(basename "$gate" _counters.sh)
  [ "$vein" = all ] && continue
  prog=$(sed -n 's/.*KANSO_COUNTERS=1 \.\/\([a-z0-9_-]*\).*/\1/p' "$gate" | head -n 1)
  golden=$(grep -o 'bench/cost_golden[a-z0-9_]*\.txt' "$gate" | head -n 1)
  if [ -z "$prog" ] || [ -z "$golden" ]; then
    echo "$gate names no counting program or no cost golden" >&2
    exit 1
  fi
  veins="$veins $vein:$prog:$golden"
done

# `--list` prints each vein and its golden without building anything, and
# tests/every_counter_gate_is_in_the_sweep.rs compares that output with the
# cost goldens on disk.
if [ "$1" = "--list" ]; then
  for row in $veins; do
    echo "${row%%:*} ${row##*:}"
  done
  exit 0
fi

sh scripts/gates/build_benchmarks.sh >/dev/null

moved=""
for row in $veins; do
  vein=${row%%:*}
  rest=${row#*:}
  prog=${rest%%:*}
  golden=${rest#*:}
  got=$(mktemp)
  want=$(mktemp)
  KANSO_COUNTERS=1 "./$prog" 2>"$got" >/dev/null
  grep -v '^#' "$golden" > "$want"
  if diff -q "$want" "$got" >/dev/null; then
    rm -f "$got" "$want"
    continue
  fi
  moved="$moved $vein"
  echo "=== $vein ($golden)"
  diff "$want" "$got" || true
  if [ "$write" -eq 1 ]; then
    # keep the header: replace the data rows in place, line for line, and
    # append any row the measured file holds beyond the golden's last
    sh scripts/gates/keep_header.sh "$golden" "$got" > "$golden.new" \
      && mv "$golden.new" "$golden"
    echo "--- rewrote $golden"
  fi
  rm -f "$got" "$want"
done

# NOT A VEIN ABOVE, and it cannot be one: the lazy tier's goldens are
# `tests/golden/mem/*.mem`, read by `tests/golden.rs`, so there is no
# `*_counters.sh` gate naming them and the loop over the gates walks straight
# past. CLAUDE.md names the .mem vein FIRST in the list a
# counter change must regenerate, and until 2026-09-06 this sweep could not see
# it. The compile sweep had the same hole in the same week, twice over.
#
# IT COST 158 SECONDS to read and past 204 to `--write`, measured 2026-09-06,
# and both figures were the whole `golden` binary rather than this vein: nine
# of its ten tests have nothing to do with the .mem files, and the two micro
# corpus tests are where the time went. Naming the one test that owns the vein
# takes 3.35 seconds, and the whole sweep now runs in 51 where it ran in about
# 200. The old numbers are kept here because they are what a reader will find
# in the log entry that recorded them.
# NAMED, not the whole binary. `--test golden` builds ten tests and this step
# reads the exit status of all ten, so ANY of them going red was reported here
# as "counters moved: mem" -- a diagnosis pointing at the one vein that had not
# moved. The two micro-corpus tests are also the slow ones: naming this test
# alone takes 3.35 seconds against 158 for the binary, measured 2026-09-06 on
# this container. `mem_corpus_pins_native_allocator_counters` is the only test
# that reads KANSO_REGEN_MEM_GOLDEN, which is why both branches name it and
# why tests/the_sweep_reads_the_mem_vein_alone.rs derives that rather than
# trusting this comment.
mem_test=mem_corpus_pins_native_allocator_counters
printf '=== lazy tier (tests/golden/mem/*.mem)\n'
if [ "$write" -eq 1 ]; then
  if KANSO_REGEN_MEM_GOLDEN=1 cargo test --release --test golden "$mem_test" >/dev/null 2>&1; then
    echo "--- regenerated the .mem vein"
  else
    echo "--- the .mem regeneration FAILED; run it directly to read why"
    moved="$moved mem"
  fi
elif out=$(cargo test --release --test golden "$mem_test" 2>&1); then
  echo "--- agrees"
else
  echo "$out" | sed 's/^/    /'
  moved="$moved mem"
fi

if [ -z "$moved" ]; then
  echo "counters: every cost vein and the lazy tier agree with their goldens"
  exit 0
fi
echo "counters moved:$moved"
[ "$write" -eq 1 ] && exit 0
echo "run with --write to regenerate them, then say why in design/compiler-log.md"
exit 1
