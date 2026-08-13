#!/bin/sh
# The book rule, enforced: every .out under docs/book/samples re-runs and
# must match. The .out basename carries the mode as a suffix:
#   name.out            kanso run name.kso   (or name/ directory-module)
#   name_check.out      kanso check name.kso
#   name_test.out       kanso test name.kso
#   name_build.out      kanso build name.kso   (run from a temp dir)
#   name_plan.out       kanso run name.kso --plan
#   name.plan.out       kanso run name.kso --plan
#   name_counters.out   KANSO_COUNTERS=1 kanso run name.kso
# A sibling name.manual file (holding one line of why) exempts an .out the
# harness cannot replay — repl transcripts, wall-clock timings, IR greps.
set -e
# panels render their samples; drift fails the build
./target/release/kanso run scripts/book_panels
# the sample .outs pin the dice; a bare run seeds from entropy
export KANSO_SEED=2685821657736338717
KANSO=$(pwd)/target/release/kanso
fail=0
for out in docs/book/samples/*/*.out; do
  base=${out%.out}
  if [ -f "$base.manual" ]; then
    continue
  fi
  mode=run; extra=""; env_prefix=""; stripped="$base"
  case "$base" in
    *_check)    mode=check; stripped="${base%_check}" ;;
    *_test)     mode=test;  stripped="${base%_test}" ;;
    *_build)    mode=build; stripped="${base%_build}" ;;
    *_plan)     mode=run; extra="--plan"; stripped="${base%_plan}" ;;
    *.plan)     mode=run; extra="--plan"; stripped="${base%.plan}" ;;
    *_counters) mode=run; env_prefix="KANSO_COUNTERS=1"; stripped="${base%_counters}" ;;
  esac
  src=""
  for cand in "$base.kso" "$stripped.kso" "$stripped" "$base"; do
    if [ -e "$cand" ] && [ "$cand" != "$out" ]; then src="$cand"; break; fi
  done
  if [ -z "$src" ]; then
    echo "NO SOURCE: $out"
    fail=1
    continue
  fi
  # run from the sample's directory with the bare name, matching how the
  # panels invoke it — diagnostics then print the clean relative path
  dir=$(dirname "$src"); name=$(basename "$src")
  if [ "$mode" = build ]; then
    tmp=$(mktemp -d)
    cp "$src" "$tmp/"
    actual=$( (cd "$tmp" && "$KANSO" build "$name" 2>&1) ) || true
    rm -rf "$tmp"
  else
    verb="$mode"
    target="$name"
    where="$dir"
    stage=""
    # a sample holding BOTH definitions and bare statements is a play file —
    # the relaxed single-file form. Definitions alone are a library, and the
    # book has a sample whose whole lesson is that `run` says so.
    if [ "$mode" = run ] && [ -f "$src" ] && ! grep -q '^pub play' "$src" \
      && grep -qE '^(fn|type) ' "$src" \
      && awk '/^[^ #]/ && !/^(import|fn|type) / { found = 1 } END { exit !found }' "$src"; then
      verb=play
    fi
    # a sample that exports `play` is a library, and what runs one is the
    # entry file that imports it. The directory comes along, because samples
    # read fixtures that sit beside them — all but a directory of the
    # sample's own name: the chapter that teaches modules has both `shop.kso`
    # and a `shop/`, and an import cannot name both.
    if [ "$mode" = run ] && [ -f "$src" ] && grep -q '^pub play' "$src"; then
      stage=$(mktemp -d)
      cp -R "$dir/." "$stage/"
      stem="${name%.kso}"
      rm -rf "$stage/$stem"
      printf 'import "%s"\n\n%s/play\n' "$stem" "$stem" > "$stage/run_$stem.kso"
      target="run_$stem.kso"
      where="$stage"
    fi
    actual=$( (cd "$where" && env $env_prefix "$KANSO" "$verb" "$target" $extra 2>&1) ) || true
    [ -n "$stage" ] && rm -rf "$stage"
  fi
  if [ "$actual" != "$(cat "$out")" ]; then
    echo "MISMATCH: $out (mode $mode)"
    fail=1
  fi
done
# The write path, on a copy. Everything above runs book_panels read-only, and
# the read path never forces the chapter text it built — `keeping` short-
# circuits on `not write` — so until now nothing exercised the one command that
# edits the shipped book, and a runaway lived there long enough to reach 580 GB
# before anybody ran it by hand.
#
# The assertion is the right one: that runaway ended in a SIGKILL, which is a
# non-zero exit, which is what the first branch below catches. What is missing
# is a fixture that provokes it. This one was measured against a compiler with
# the bug restored and finished in 18 MB, because the escaping the runaway
# lived in is quadratic in the body it escapes and no recorded output in the
# book is larger than 598 bytes. The reduced chapter from that investigation
# belongs in the corpus, and until it is there this watches that the write path
# runs and puts a drifted panel back, which is more than nothing watched it
# before.
#
# A recorded output is removed as well as a body staled, because `no_recorded`
# is the branch that reaches the escaping, and staling a body alone leaves it
# untaken.
#
# Native, deliberately: the oracle never had the runaway, so asking it would
# watch the one engine that was fine.
here=$(pwd)
scratch=$(mktemp -d)
mkdir -p "$scratch/docs/book"
(cd docs/book && tar cf - .) | (cd "$scratch/docs/book" && tar xf -)
python3 scripts/stale_a_panel.py "$scratch/docs/book/ch03.html" \
  "$scratch/docs/book/samples/ch03"
if ! (cd "$scratch" && "$KANSO" run "$here/scripts/book_panels" -- --write \
      >"$scratch/log" 2>&1); then
  echo "WRITE PATH: book_panels --write did not finish"
  tail -5 "$scratch/log"
  fail=1
elif ! grep -q 'panel(s) rewritten' "$scratch/log"; then
  echo "WRITE PATH: nothing was rewritten, so the fixture proved nothing"
  tail -3 "$scratch/log"
  fail=1
elif grep -q STALE "$scratch/docs/book/ch03.html"; then
  echo "WRITE PATH: the staled panel was not rewritten"
  fail=1
else
  echo "book panels: the write path rewrites a staled panel back"
fi
rm -rf "$scratch"

[ "$fail" = 0 ] && echo "book samples: all outputs verified"
exit $fail
