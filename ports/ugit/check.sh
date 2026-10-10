#!/bin/sh
# Runs the unit tests, then every fixture three ways -- on the interpreter,
# as a dev-tier native binary and as a release binary -- and fails if any
# output differs from the expected file or from the other engines.
#
#   sh check.sh            check everything
#   sh check.sh --bless    rewrite the expected files from the interpreter
#   sh check.sh NAME...    run only the named fixtures
#
# A fixture is fixtures/cases/NAME.sh, a shell session that drives `ugit`
# in an empty scratch directory made fresh for each engine. KANSO_NOW pins
# the clock so commit ids are the same every run; `tick` moves it on a
# minute. A ugit command that exits non-zero prints `[exit N]` after its
# output. Its expected output, fixtures/cases/NAME.out, holds stdout and
# stderr together.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
ROOT=$(cd "$(dirname "$0")" && pwd)
CASES="$ROOT/fixtures/cases"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
BLESS=0
if [ "${1:-}" = "--bless" ]; then
  BLESS=1
  shift
fi
fail=0

echo "== unit tests"
for m in delta store ugit; do
  "$KANSO" test "$ROOT/$m" > "$WORK/unit-$m.txt" 2>&1 || fail=1
  printf '%s: ' "$m"
  tail -1 "$WORK/unit-$m.txt"
  grep FAILED "$WORK/unit-$m.txt"
done

echo "== build"
mkdir -p "$WORK/dev" "$WORK/release"
(cd "$WORK/dev" && "$KANSO" build "$ROOT" > /dev/null) || exit 1
(cd "$WORK/release" && "$KANSO" build "$ROOT" --release > /dev/null) || exit 1

# One engine runs one fixture in a directory of its own.
run_case() {
  engine=$1
  script=$2
  out=$3
  dir="$WORK/run-$engine"
  mkdir "$dir"
  (
    cd "$dir" || exit 1
    KANSO_NOW=1700000000000
    UGIT_AUTHOR="A U Thor <author@example.com>"
    export KANSO_NOW UGIT_AUTHOR
    tick() {
      KANSO_NOW=$((KANSO_NOW + 60000))
      export KANSO_NOW
    }
    case $engine in
      interp)  run() { "$KANSO" run "$ROOT" --interp -- "$@"; } ;;
      dev)     run() { "$WORK/dev/ugit" "$@"; } ;;
      release) run() { "$WORK/release/ugit" "$@"; } ;;
    esac
    ugit() {
      run "$@"
      status=$?
      [ "$status" -ne 0 ] && echo "[exit $status]"
      return 0
    }
    . "$script"
  ) > "$out" 2>&1 < /dev/null
  rm -rf "$dir"
}

echo "== fixtures"
total=0
if [ $# -gt 0 ]; then
  names="$*"
else
  names=$(cd "$CASES" && ls *.sh | sed 's/\.sh$//')
fi
for name in $names; do
  total=$((total + 1))
  for engine in interp dev release; do
    run_case "$engine" "$CASES/$name.sh" "$WORK/$name.$engine"
  done
  if [ "$BLESS" = 1 ]; then
    cp "$WORK/$name.interp" "$CASES/$name.out"
  fi
  for engine in interp dev release; do
    if ! cmp -s "$WORK/$name.$engine" "$CASES/$name.out"; then
      echo "FAIL $name ($engine differs from $name.out)"
      diff "$CASES/$name.out" "$WORK/$name.$engine" | head -20
      fail=1
    fi
  done
done

if [ "$fail" = 0 ]; then
  echo "ok: $total fixtures agree on interp, dev and release"
else
  echo "check.sh: FAILED"
fi
exit "$fail"
