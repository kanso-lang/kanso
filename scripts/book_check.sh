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
python3 scripts/book_panels.py --check
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
    if [ "$mode" = run ] && grep -q "pub play" "$dir/$name" 2>/dev/null; then verb=play; fi
    actual=$( (cd "$dir" && env $env_prefix "$KANSO" "$verb" "$name" $extra 2>&1) ) || true
  fi
  if [ "$actual" != "$(cat "$out")" ]; then
    echo "MISMATCH: $out (mode $mode)"
    fail=1
  fi
done
[ "$fail" = 0 ] && echo "book samples: all outputs verified"
exit $fail
