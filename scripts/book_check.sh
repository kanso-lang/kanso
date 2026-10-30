#!/bin/sh
# The book rule, enforced: every .out under docs/book/samples re-runs and
# must match. The .out basename carries the mode as a suffix:
#   name.out            kanso run name.kso   (or name/ directory-module)
#   name_check.out      kanso check name.kso
#   name_test.out       kanso test name.kso
#   name_build.out      kanso build name.kso   (run from a temp dir)
#   name_plan.out       kanso run name.kso --plan
#   name_counters.out   KANSO_COUNTERS=1 kanso run name.kso
#   name@slug.out       kanso run name.kso, given the words in name@slug.args
#   name_install.out    kanso install name   (and _list, _update the same way)
# A sibling name.manual file (holding one line of why) exempts an .out the
# harness cannot replay — repl transcripts, wall-clock timings, IR greps.
# BOOK_WRITE=1 rewrites each .out that differs from what its sample now does,
# for a change that moves outputs on purpose; read the diff before keeping it.
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
# The hako cache is state no sample produced, so every run starts from an
# empty one. A sample that depends on a hako commits its hako.lock, and the
# hakos it names are installed before any panel runs: the recordings are read
# in name order, and a program's run sorts ahead of its install.
KANSO_HAKO=$(mktemp -d)
export KANSO_HAKO
for lock in $(find docs/book/samples -name hako.lock | sort); do
  dir=$(dirname "$lock")
  stage=$(mktemp -d)
  cp -R "$dir/." "$stage/"
  if ! (cd "$stage" && "$KANSO" install . >/dev/null 2>"$stage.err"); then
    echo "HAKO: $dir does not install: $(cat "$stage.err")"
    fail=1
  fi
  rm -rf "$stage" "$stage.err"
done
# How a reader runs a sample file. `run` takes a file of statements alone and
# `play` one with definitions beside its statements; a file of definitions
# and constants alone is a library, and a library that exports `play` runs
# only from an entry that imports it, which no reader types. A statement is a
# top-level line that is neither a declaration nor a binding.
kind_of() {
  if grep -q '^pub play' "$1"; then echo stub
  elif ! grep -qE '^(pub )?(fn|type) ' "$1"; then echo run
  elif awk '/^[^ #]/ && !/^(import|pub|fn|type|derive) / && !/^[a-z_][a-z0-9_]* =( |$)/ { found = 1 } END { exit !found }' "$1"; then echo play
  else echo library
  fi
}
for out in docs/book/samples/*/*.out; do
  base=${out%.out}
  if [ -f "$base.manual" ]; then
    continue
  fi
  mode=run; extra=""; env_prefix=""; stripped="$base"; given=""
  # name@slug.out is the run its panel's title gives words to, `-- clay` or
  # `< rain.txt`; name@slug.args holds the words as the title wrote them
  case "$base" in
    *@*) given=$(cat "$base.args"); base="${base%%@*}"; stripped="$base" ;;
  esac
  case "$base" in
    *_check)    mode=check; stripped="${base%_check}" ;;
    *_test)     mode=test;  stripped="${base%_test}" ;;
    *_build)    mode=build; stripped="${base%_build}" ;;
    # a hako verb runs on a directory, and only when no sample has the whole
    # name: values/long_list.out is a run of long_list.kso
    *_install|*_list|*_update)
      if [ ! -e "$base.kso" ] && [ ! -e "$base" ]; then
        mode=${base##*_}; stripped="${base%_*}"
      fi ;;
    *_plan)     mode=run; extra="--plan"; stripped="${base%_plan}" ;;
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
    # Every sample runs in a fresh copy of its directory, under the same
    # name. Samples write files (the failure chapter's ordered.kso leaves slow.txt, which
    # grouped.kso must not find), so running them in place made a second
    # run read what the first one left behind.
    stage=$(mktemp -d)
    where="$stage/$(basename "$dir")"
    mkdir "$where"
    cp -R "$dir/." "$where/"
    # a link into lib/ is the library itself, which may use builtins only
    # from where it lives, so the copy keeps it a link, made absolute
    find "$dir" -type l | while read -r link; do
      ln -sfn "$(readlink -f "$link")" "$where/${link#"$dir"/}"
    done
    kind=library
    [ -f "$src" ] && kind=$(kind_of "$src")
    [ "$mode" = run ] && [ "$kind" = play ] && verb=play
    # the entry a `play` export needs, in a stage without a directory of the
    # sample's own name: the chapter that teaches modules has both `shop.kso`
    # and a `shop/`, and an import cannot name both.
    if [ "$mode" = run ] && [ "$kind" = stub ]; then
      stem="${name%.kso}"
      rm -rf "$where/$stem"
      printf 'import "./%s"\n\n%s/play\n' "$stem" "$stem" > "$where/run_$stem.kso"
      target="run_$stem.kso"
    fi
    actual=$( (cd "$where" && eval "env $env_prefix \"\$KANSO\" \"\$verb\" \"\$target\" $extra $given" 2>&1) ) || true
    rm -rf "$stage"
  fi
  if [ "$actual" != "$(cat "$out")" ] && [ -n "$BOOK_WRITE" ]; then
    printf '%s\n' "$actual" > "$out"
    echo "rewrote: $out (mode $mode)"
  elif [ "$actual" != "$(cat "$out")" ]; then
    echo "MISMATCH: $out (mode $mode)"
    fail=1
  fi
done
# A panel's title is the command a reader will type, so it has to be the
# command this harness ran. For years it was not: files with definitions ran
# under `play` while their panels said `kanso run`, and a reader typing the
# title got "is a library — nothing to run". A `play` export has no command at
# all; the samples named here keep one until the toolchain can run them
# another way (#2204: `play --plan`; #2205: named groups, constants and
# siblings in play files).
waiting=" appb/timing.kso effects/count.kso flow/concurrency.kso "
waiting="$waiting flow/accumulate.kso flow/dice.kso "
waiting="$waiting flow/join.kso modules/shop.kso json/escapes.kso "
titles=$(for page in docs/book/*.html; do
  ch=$(basename "$page" .html)
  grep -oE 'code-panel-title">(KANSO_COUNTERS=1 )?kanso (run|play) [a-z_0-9/]+\.kso' "$page" \
    | sed -E 's/.*kanso (run|play) /\1 /' | while read -r said f; do
    src=docs/book/samples/$ch/$f
    [ -f "$src" ] || src=$(find "docs/book/samples/$ch" -path "*/$f" | head -1)
    [ -f "$src" ] || continue
    kind=$(kind_of "$src")
    case "$said/$kind" in
      run/run|run/library|play/play|play/run) ;;
      */stub) case "$waiting" in
          *" $ch/${f##*/} "*) ;;
          *) echo "TITLE: $page says \`kanso $said $f\`, a library exporting play" ;;
        esac ;;
      *) echo "TITLE: $page says \`kanso $said $f\`; a reader runs it with \`kanso $kind\`" ;;
    esac
  done
done)
if [ -n "$titles" ]; then
  echo "$titles"
  fail=1
fi
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
# the site pages book_panels also reads, the examples they quote, and the
# goldens their output panels show
cp docs/index.html docs/about.html "$scratch/docs/"
cp -R examples "$scratch/examples"
mkdir -p "$scratch/tests/golden"
cp -R tests/golden/examples tests/golden/errors "$scratch/tests/golden/"
"$KANSO" run scripts/stale_a_panel -- "$scratch/docs/book/failure.html"
if ! (cd "$scratch" && "$KANSO" run "$here/scripts/book_panels" -- --write \
      >"$scratch/log" 2>&1); then
  echo "WRITE PATH: book_panels --write did not finish"
  tail -5 "$scratch/log"
  fail=1
elif ! grep -q '2 panel(s) rewritten' "$scratch/log"; then
  echo "WRITE PATH: both staled panels were not rewritten"
  tail -3 "$scratch/log"
  fail=1
elif grep -q STALE "$scratch/docs/book/failure.html"; then
  echo "WRITE PATH: a staled panel was not put back"
  fail=1
else
  echo "book panels: the write path rewrites both staled panels back"
fi
rm -rf "$scratch"

[ "$fail" = 0 ] && echo "book samples: all outputs verified"
exit $fail
