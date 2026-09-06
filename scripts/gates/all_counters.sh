#!/bin/sh
# Every counter gate at once, and the twelve goldens they read.
#
# Each is its own file, so following the rule in CLAUDE.md -- "counters changed,
# regenerate every vein in the same PR" -- meant remembering a filename apiece
# and which golden each one writes. There were nine when this was written; a
# branch that changed how closures are built regenerated the .mem vein, the four
# code goldens and the wasm blob and missed all nine, and CI found them one round
# late, which cost eight minutes for something a container answers in twenty
# seconds. The count is TWELVE now and it is not written down anywhere but the
# table below, which is what a spec reads.
#
# It does NOT stop at the first divergence. Each vein is an independent
# dimension of the same run, and the CI job that owns them says why: stopping
# early once kept the instruction counts from being measured at all for two
# runs, and the decoder was 12.2% slower the whole time with nothing able to
# say so. The summary at the end names every vein that moved.
#
#   sh scripts/gates/all_counters.sh           read-only, one summary
#   sh scripts/gates/all_counters.sh --write   regenerate each golden that moved
#
# --write preserves each golden's comment header. Two of them have one --
# cost_golden_wide.txt and cost_golden_pend.txt -- and a plain `cp` of the
# measured file over the golden strips it, which is a trap worth holding here
# rather than remembering.
set -e
write=0
[ "$1" = "--write" ] && write=1

sh scripts/gates/build_benchmarks.sh >/dev/null

# vein:program:golden
veins="decode:jsonbench:bench/cost_golden.txt
encode:encodebench:bench/cost_golden_encode.txt
oneshot:oneshot:bench/cost_golden_oneshot.txt
basket:basket:bench/cost_golden_basket.txt
wide:widebench:bench/cost_golden_wide.txt
pend:pendbench:bench/cost_golden_pend.txt
escape:escapebench:bench/cost_golden_escape.txt
digest:digestbench:bench/cost_golden_digest.txt
read:readbench:bench/cost_golden_read.txt
scan:scanbench:bench/cost_golden_scan.txt
live:livebench:bench/cost_golden_live.txt
run:runbench:bench/cost_golden_run.txt"

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
    # keep the header: replace the data rows in place, line for line
    awk -v got="$got" '
      /^#/ || /^[[:space:]]*$/ { print; next }
      { if ((getline line < got) > 0) print line; else print }
    ' "$golden" > "$golden.new" && mv "$golden.new" "$golden"
    echo "--- rewrote $golden"
  fi
  rm -f "$got" "$want"
done

# NOT A ROW IN THE TABLE ABOVE, and it cannot be one: the lazy tier's goldens
# are `tests/golden/mem/*.mem`, read by `tests/golden.rs`, so there is no
# `*_counters.sh` gate naming them and the derivation the table is pinned to
# walks straight past. CLAUDE.md names the .mem vein FIRST in the list a
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
  echo "counters: the twelve cost veins and the lazy tier agree with their goldens"
  exit 0
fi
echo "counters moved:$moved"
[ "$write" -eq 1 ] && exit 0
echo "run with --write to regenerate them, then say why in design/compiler-log.md"
exit 1
