#!/bin/sh
# Runs the unit tests, then every fixture three ways -- on the interpreter,
# as a dev-tier native binary and as a release binary -- and fails if any
# output differs from the expected file or from the other engines.
#
#   sh check.sh            check everything
#   sh check.sh --bless    rewrite the expected files from the interpreter
#   sh check.sh NAME...    run only the named fixtures
#
# A fixture is fixtures/cases/NAME.cmd, a few lines of shell in which `kv`
# is the program under test and $STORE is an empty directory for it to keep
# a store in. Its expected output, fixtures/cases/NAME.out, holds stdout and
# stderr together, then a last line `[exit N]` for the last command.
#
# Helpers a fixture can call:
#   bytes FILE          the file as hex, sixteen bytes to a line (od)
#   tear FILE N         keep only the first N bytes, as a crash would
#   poke FILE AT HEX    overwrite the byte at offset AT
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
for m in bitcask cli; do
  "$KANSO" test "$ROOT/$m" > "$WORK/unit-$m.txt" 2>&1 || fail=1
  grep -E "passed|FAILED|error" "$WORK/unit-$m.txt"
done

echo "== build"
mkdir -p "$WORK/dev" "$WORK/release"
(cd "$WORK/dev" && "$KANSO" build "$ROOT" > /dev/null) || exit 1
(cd "$WORK/release" && "$KANSO" build "$ROOT" --release > /dev/null) || exit 1

run_case() {
  engine=$1
  cmd=$2
  out=$3
  store="$WORK/store-$engine"
  rm -rf "$store"
  mkdir -p "$store"
  (
    cd "$store" || exit 1
    STORE=db
    export STORE
    case $engine in
      interp)  kv() { "$KANSO" run "$ROOT" --interp -- "$@"; } ;;
      dev)     kv() { "$WORK/dev/kv" "$@"; } ;;
      release) kv() { "$WORK/release/kv" "$@"; } ;;
    esac
    bytes() { od -An -tx1 -v "$1"; }
    tear() { head -c "$2" "$1" > "$1.torn" && mv "$1.torn" "$1"; }
    poke() {
      printf "\\$(printf '%03o' "0x$3")" |
        dd of="$1" bs=1 seek="$2" conv=notrunc 2> /dev/null
    }
    eval "$cmd" < /dev/null
  ) > "$out" 2>&1
  echo "[exit $?]" >> "$out"
}

echo "== fixtures"
total=0
if [ $# -gt 0 ]; then
  names=$*
else
  names=$(cd "$CASES" && ls *.cmd | sed 's/\.cmd$//')
fi
for name in $names; do
  cmd=$(cat "$CASES/$name.cmd")
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
