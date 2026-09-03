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
# and the panels book_panels cannot resolve — the ones quoting a file this
# repository owns, or a directory module — are checked structurally instead
./target/release/kanso run scripts/book_quotes
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
      printf 'import "./%s"\n\n%s/play\n' "$stem" "$stem" > "$stage/run_$stem.kso"
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
# circuits on `not write` — so nothing exercised the one command that edits the
# shipped book.
#
# Both panel shapes are staled. A source panel is regenerated from the .kso; a
# recorded-output panel is compared against the .out and rewritten from it,
# which is the branch that escapes a recorded body into html. The two reach
# different code, and a chapter carrying both is what this needs.
#
# What this does not catch is the runaway that put 580 GB through the write
# path. Measured against a compiler with that bug restored, book_panels --write
# peaks at 16 to 19 MB here — over the staled book, over a chapter truncated
# mid-panel, and over recorded bodies grown to 10, 20, 43 and 82 KB. The
# escaping is not where that bug bites: it is an accumulator handed through an
# intermediate function, and that shape is pinned in tests/golden/mem.
#
# Native, deliberately: the oracle never had the runaway, so asking it would
# watch the one engine that was fine.
here=$(pwd)
scratch=$(mktemp -d)
mkdir -p "$scratch/docs/book"
(cd docs/book && tar cf - .) | (cd "$scratch/docs/book" && tar xf -)
"$KANSO" run scripts/stale_a_panel -- "$scratch/docs/book/ch04.html"
if ! (cd "$scratch" && "$KANSO" run "$here/scripts/book_panels" -- --write \
      >"$scratch/log" 2>&1); then
  echo "WRITE PATH: book_panels --write did not finish"
  tail -5 "$scratch/log"
  fail=1
elif ! grep -q '2 panel(s) rewritten' "$scratch/log"; then
  echo "WRITE PATH: both staled panels were not rewritten"
  tail -3 "$scratch/log"
  fail=1
elif grep -q STALE "$scratch/docs/book/ch04.html"; then
  echo "WRITE PATH: a staled panel was not put back"
  fail=1
else
  echo "book panels: the write path rewrites both staled panels back"
fi
rm -rf "$scratch"

[ "$fail" = 0 ] && echo "book samples: all outputs verified"
exit $fail
