#!/bin/sh
# Runs the unit tests, then every fixture on three engines: the interpreter
# (kanso run --interp), the dev build and the release build. Each run's
# stdout, stderr, exit status and the files it created, changed or removed
# are written as one transcript; the three transcripts must agree with each
# other and with the fixture's `expected` file.
#
#   sh check.sh            check everything
#   sh check.sh --update   rewrite each `expected` from the interpreter
#   sh check.sh --gnu      also run GNU make (as `make -rR`) on every fixture
#                          and show where it differs from `expected`
#
# A fixture is a directory under fixtures/ holding `cmd` (the arguments, one
# line, shell-quoted), `in/` (the working directory the command runs in),
# optionally `times`, and `expected`.
#
# std/os cannot read a file's modification time, so the port asks stat(1),
# and the fixtures need times that do not depend on when `cp` ran. Every
# file copied from `in/` is stamped 2001-09-09 (epoch 1000000000); `times`
# then sets particular files, one `path epoch-seconds` pair per line. A file
# a recipe writes gets the real time, which is always later.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
mode=${1:-}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failed=0

say() { printf '%s\n' "$*"; }

say "== unit tests"
for module in words glob expand reader; do
  if ! "$KANSO" test "$here/$module" > "$work/test.out" 2>&1; then
    cat "$work/test.out"
    say "FAIL: kanso test $module"
    failed=1
  else
    tail -1 "$work/test.out" | sed "s/^/$module: /"
  fi
done

say "== building"
mkdir -p "$work/dev" "$work/rel"
(cd "$work/dev" && "$KANSO" build "$here" > /dev/null) || { say "dev build failed"; exit 1; }
(cd "$work/rel" && "$KANSO" build "$here" --release > /dev/null) || { say "release build failed"; exit 1; }

# the files under $1 that differ from those under $2, then the ones gone
changes() {
  (cd "$1" && find . -type f | sort) | while read -r f; do
    if [ ! -f "$2/$f" ] || ! cmp -s "$1/$f" "$2/$f"; then
      printf '== %s\n' "$f"
      cat "$1/$f"
      printf '\n'
    fi
  done
  (cd "$2" && find . -type f | sort) | while read -r f; do
    [ -f "$1/$f" ] || printf '== %s (removed)\n' "$f"
  done
}

stage() { # fixture-dir run-dir
  rm -rf "$2"
  mkdir -p "$2"
  if [ -d "$1/in" ]; then cp -R "$1/in/." "$2/"; fi
  (cd "$2" && find . -exec touch -h -d @1000000000 {} +)
  if [ -f "$1/times" ]; then
    while read -r path stamp; do
      [ -n "$path" ] && touch -h -d "@$stamp" "$2/$path"
    done < "$1/times"
  fi
}

run_one() { # engine fixture-dir transcript
  engine=$1
  fx=$2
  transcript=$3
  run="$work/run"
  stage "$fx" "$run"
  input=/dev/null
  [ -f "$fx/stdin" ] && input="$fx/stdin"
  eval "set -- $(cat "$fx/cmd")"
  case "$engine" in
    interp) (cd "$run" && "$KANSO" run "$here" --interp -- "$@" < "$input" > "$work/out" 2> "$work/err") ;;
    dev) (cd "$run" && "$work/dev/make" "$@" < "$input" > "$work/out" 2> "$work/err") ;;
    rel) (cd "$run" && "$work/rel/make" "$@" < "$input" > "$work/out" 2> "$work/err") ;;
    gnu) (cd "$run" && make -rR "$@" < "$input" > "$work/out" 2> "$work/err") ;;
  esac
  status=$?
  {
    printf '$ make %s\n' "$(cat "$fx/cmd")"
    cat "$work/out"
    printf -- '--- stderr\n'
    cat "$work/err"
    printf -- '--- exit %s\n' "$status"
    if [ -d "$fx/in" ]; then changes "$run" "$fx/in"; else changes "$run" "$work/empty"; fi
  } > "$transcript"
}

mkdir -p "$work/empty"
say "== fixtures"
count=0
gnu_differs=0
for fixture in "$here"/fixtures/*/; do
  fixture=${fixture%/}
  name=$(basename "$fixture")
  count=$((count + 1))
  for engine in interp dev rel; do
    run_one "$engine" "$fixture" "$work/$engine.txt"
  done
  if [ "$mode" = "--update" ]; then
    cp "$work/interp.txt" "$fixture/expected"
  fi
  bad=""
  cmp -s "$work/interp.txt" "$work/dev.txt" || bad="$bad dev"
  cmp -s "$work/interp.txt" "$work/rel.txt" || bad="$bad rel"
  cmp -s "$work/interp.txt" "$fixture/expected" || bad="$bad expected"
  if [ -n "$bad" ]; then
    say "FAIL $name: interpreter disagrees with$bad"
    for other in dev rel; do
      cmp -s "$work/interp.txt" "$work/$other.txt" || diff "$work/interp.txt" "$work/$other.txt" | sed "s/^/  $other: /"
    done
    diff "$fixture/expected" "$work/interp.txt" | sed 's/^/  expected: /'
    failed=1
  fi
  if [ "$mode" = "--gnu" ]; then
    run_one gnu "$fixture" "$work/gnu.txt"
    if ! cmp -s "$fixture/expected" "$work/gnu.txt"; then
      gnu_differs=$((gnu_differs + 1))
      say "GNU make differs on $name:"
      diff "$fixture/expected" "$work/gnu.txt" | sed 's/^/  gnu: /'
    fi
  fi
done
say "$count fixtures, 3 engines each"
[ "$mode" = "--gnu" ] && say "GNU make 4.3 differs on $gnu_differs of them"

if [ $failed -ne 0 ]; then
  say "check.sh: FAILED"
  exit 1
fi
say "check.sh: all engines agree with every expected file"
