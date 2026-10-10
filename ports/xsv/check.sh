#!/bin/sh
# Runs the unit tests, then every fixture three ways -- on the interpreter,
# as a dev-tier native binary and as a release binary -- and fails if any
# output differs from the expected file or from the other engines.
#
#   sh check.sh            check everything
#   sh check.sh --bless    rewrite the expected files from the interpreter
#
# A fixture is fixtures/cases/NAME.cmd, one shell command line in which `xsv`
# is the program under test, run from fixtures/data. Its expected output,
# fixtures/cases/NAME.out, holds stdout and stderr together, then a last
# line `[exit N]`. `$OUT` names a scratch directory for commands that write
# files.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
ROOT=$(cd "$(dirname "$0")" && pwd)
CASES="$ROOT/fixtures/cases"
DATA="$ROOT/fixtures/data"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
BLESS=0
[ "${1:-}" = "--bless" ] && BLESS=1
fail=0

echo "== unit tests"
"$KANSO" test "$ROOT/csv" > "$WORK/unit.txt" 2>&1 || fail=1
"$KANSO" test "$ROOT/xsv" >> "$WORK/unit.txt" 2>&1 || fail=1
grep -E "passed|FAILED" "$WORK/unit.txt"

echo "== build"
mkdir -p "$WORK/dev" "$WORK/release"
(cd "$WORK/dev" && "$KANSO" build "$ROOT") || exit 1
(cd "$WORK/release" && "$KANSO" build "$ROOT" --release) || exit 1

# One engine runs one fixture. The engine is a function so that a pipeline
# inside a command line uses the same engine at every stage.
run_case() {
  engine=$1
  cmd=$2
  out=$3
  mkdir -p "$WORK/out-$engine"
  (
    cd "$DATA" || exit 1
    OUT="$WORK/out-$engine"
    export OUT
    case $engine in
      interp)  xsv() { "$KANSO" run "$ROOT" --interp -- "$@"; } ;;
      dev)     xsv() { "$WORK/dev/xsv" "$@"; } ;;
      release) xsv() { "$WORK/release/xsv" "$@"; } ;;
    esac
    eval "$cmd" < /dev/null
  ) > "$out" 2>&1
  echo "[exit $?]" >> "$out"
}

echo "== fixtures"
total=0
for f in "$CASES"/*.cmd; do
  name=$(basename "$f" .cmd)
  cmd=$(cat "$f")
  total=$((total + 1))
  for engine in interp dev release; do
    run_case "$engine" "$cmd" "$WORK/$name.$engine"
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
  if ! cmp -s "$WORK/$name.interp" "$WORK/$name.dev" ||
     ! cmp -s "$WORK/$name.interp" "$WORK/$name.release"; then
    echo "FAIL $name (engines disagree)"
    fail=1
  fi
done

if [ "$fail" = 0 ]; then
  echo "ok: $total fixtures agree on interp, dev and release"
else
  echo "check.sh: FAILED"
fi
exit "$fail"
