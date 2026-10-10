#!/bin/sh
# Runs the unit tests, then every fixture on three engines: the interpreter
# (`kanso run --interp`), a dev build and a release build. Each case's
# stdout, stderr, exit status and output tree must match the expected files
# under fixtures/expected/<case>/, and so must agree across the engines.
#
#   sh check.sh            check
#   sh check.sh --bless    rewrite fixtures/expected from the interpreter,
#                          which is the oracle; review the diff afterwards
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
HERE=$(cd "$(dirname "$0")" && pwd)
cd "$HERE"
BLESS=${1:-}
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
fail=0

say() { printf '%s\n' "$*"; }

say "== unit tests"
for m in textkit toml markdown tera zola; do
  if ! "$KANSO" test "$m" > "$WORK/test-$m.log" 2>&1; then
    say "FAIL unit tests in $m"; cat "$WORK/test-$m.log"; fail=1
  else
    say "ok   $m: $(tail -1 "$WORK/test-$m.log")"
  fi
done

say "== builds"
mkdir -p "$WORK/dev" "$WORK/release"
(cd "$WORK/dev" && "$KANSO" build "$HERE/main.kso" > build.log 2>&1) \
  || { say "FAIL dev build"; cat "$WORK/dev/build.log"; exit 1; }
(cd "$WORK/release" && "$KANSO" build "$HERE/main.kso" --release > build.log 2>&1) \
  || { say "FAIL release build"; cat "$WORK/release/build.log"; exit 1; }
say "ok   dev and release binaries"

# One case per line: a name, then the program's arguments. OUT stands for
# the output directory of a build.
CASES="
blog         build fixtures/blog -o OUT
blog-check   check fixtures/blog
plain        build fixtures/plain -o OUT
plain-drafts build fixtures/plain -o OUT --drafts -u http://localhost:1111
"
for d in fixtures/errors/*/; do
  name=$(basename "$d")
  CASES="$CASES
error-$name check fixtures/errors/$name"
done
for f in fixtures/markdown/*.md; do
  name=$(basename "$f" .md)
  CASES="$CASES
markdown-$name markdown $f"
done

run_engine() { # engine case args...  (shell functions share variables,
  # so these names are not used anywhere else)
  run_eng=$1; run_dir="$WORK/$1/$2"; shift 2
  mkdir -p "$run_dir"
  run_args=$(printf '%s\n' "$*" | sed "s#OUT#$run_dir/public#")
  case $run_eng in
    interp)  "$KANSO" run main.kso --interp -- $run_args ;;
    dev)     "$WORK/dev/main" $run_args ;;
    release) "$WORK/release/main" $run_args ;;
  esac > "$run_dir/stdout" 2> "$run_dir/stderr"
  echo $? > "$run_dir/status"
}

compare() { # what expected actual
  if [ -d "$2" ] || [ -d "$3" ]; then
    diff -r "$2" "$3" > "$WORK/diff" 2>&1
  else
    diff "$2" "$3" > "$WORK/diff" 2>&1
  fi || { say "FAIL $1"; head -20 "$WORK/diff"; fail=1; return 1; }
}

printf '%s\n' "$CASES" | while read -r case args; do
  [ -z "$case" ] && continue
  for engine in interp dev release; do
    run_engine "$engine" "$case" $args
  done
  want="fixtures/expected/$case"
  if [ "$BLESS" = "--bless" ]; then
    rm -rf "$want"; mkdir -p "$want"
    for part in stdout stderr status; do
      cp "$WORK/interp/$case/$part" "$want/$part"
    done
    [ -d "$WORK/interp/$case/public" ] && cp -R "$WORK/interp/$case/public" "$want/public"
  fi
  ok=1
  for engine in interp dev release; do
    got="$WORK/$engine/$case"
    for part in stdout stderr status; do
      compare "$case ($engine) $part" "$want/$part" "$got/$part" || ok=0
    done
    if [ -d "$want/public" ] || [ -d "$got/public" ]; then
      compare "$case ($engine) output tree" "$want/public" "$got/public" || ok=0
    fi
  done
  # The engines must also agree with each other, whatever the expectation.
  for engine in dev release; do
    for part in stdout stderr status; do
      compare "$case: interp and $engine disagree on $part" \
        "$WORK/interp/$case/$part" "$WORK/$engine/$case/$part" || ok=0
    done
  done
  [ $ok = 1 ] && say "ok   $case" || echo fail >> "$WORK/failed"
done

# A generated site of 150 posts: big enough to catch what only shows at
# scale (a quadratic copy and a native stack overflow both did, see
# FRICTION F27). There is no expected tree; the engines must agree with each
# other and write the expected number of files: 150 posts, the home page,
# 75 blog pagers and the page/1 redirect, 12 tag pages, the categories
# list, robots.txt, sitemap.xml, atom.xml and 404.html.
say "== scale"
site="$WORK/scale-site"
mkdir -p "$site/content/blog"
cp -R fixtures/blog/config.toml fixtures/blog/templates "$site/"
cp fixtures/blog/content/_index.md "$site/content/"
cp fixtures/blog/content/blog/_index.md "$site/content/blog/"
i=1
while [ $i -le 150 ]; do
  {
    printf '+++\ntitle = "Post %d"\ndate = 2023-%02d-%02d\n' $i $((i % 12 + 1)) $((i % 28 + 1))
    printf '[taxonomies]\ntags = ["t%d", "all"]\n+++\n\n' $((i % 10))
    printf '## Part one\n\nSome *text* with [a link](https://x.org/%d) and `code`.\n\n' $i
    printf -- '- item one\n- item two\n\n## Part two\n\n> quoted %d\n' $i
  } > "$site/content/blog/post-$i.md"
  i=$((i + 1))
done
for engine in interp dev release; do
  run_engine "$engine" scale build "$site" -o OUT
done
scale_ok=1
for engine in dev release; do
  compare "scale: interp and $engine output trees" \
    "$WORK/interp/scale/public" "$WORK/$engine/scale/public" || scale_ok=0
  compare "scale: interp and $engine stdout" \
    "$WORK/interp/scale/stdout" "$WORK/$engine/scale/stdout" || scale_ok=0
done
files=$(find "$WORK/interp/scale/public" -type f | wc -l | tr -d ' ')
[ "$files" = 244 ] || { say "FAIL scale: $files files, expected 244"; scale_ok=0; }
[ $scale_ok = 1 ] && say "ok   scale ($files files)" || fail=1

[ -f "$WORK/failed" ] && fail=1
if [ $fail = 0 ]; then
  say "== all fixtures agree on interp, dev and release"
else
  say "== FAILURES"
fi
exit $fail
