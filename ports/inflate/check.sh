#!/bin/sh
# Everything: the unit tests, then every fixture on all three engines.
#
#   sh check.sh           run everything
#   sh check.sh --bless   rewrite the expected outputs from the interpreter
#
# A fixture is fixtures/cases/NAME.cmd, one shell command line run from the
# project directory, in which `inflate` is the program under test and `$OUT`
# is a scratch directory for files it writes. NAME.out holds what the line
# prints, stdout and stderr together, then `[exit N]`. Each fixture runs on
# the interpreter, on a dev-tier binary and on a release binary, and all
# three must match NAME.out byte for byte.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
ROOT=$(cd "$(dirname "$0")" && pwd)
CASES="$ROOT/fixtures/cases"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
BLESS=0
[ "${1:-}" = "--bless" ] && BLESS=1
fail=0

echo "== unit tests"
for module in checksum deflate container compress cli; do
  if ! (cd "$ROOT" && "$KANSO" test "$module") > "$WORK/unit-$module.txt" 2>&1; then
    fail=1
    grep -v " ok$" "$WORK/unit-$module.txt"
  fi
  echo "$module: $(tail -1 "$WORK/unit-$module.txt")"
done

echo "== build"
mkdir -p "$WORK/dev" "$WORK/release"
(cd "$WORK/dev" && "$KANSO" build "$ROOT" > /dev/null) || exit 1
(cd "$WORK/release" && "$KANSO" build "$ROOT" --release > /dev/null) || exit 1

run_case() {
  engine=$1
  cmd=$2
  out=$3
  scratch="$WORK/out-$engine"
  mkdir -p "$scratch"
  (
    cd "$ROOT" || exit 1
    OUT=$scratch
    export OUT
    case $engine in
      interp)  inflate() { "$KANSO" run "$ROOT" --interp -- "$@"; } ;;
      dev)     inflate() { "$WORK/dev/inflate" "$@"; } ;;
      release) inflate() { "$WORK/release/inflate" "$@"; } ;;
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
      echo "FAIL $name: $engine differs from $name.out"
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
