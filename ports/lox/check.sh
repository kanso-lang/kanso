#!/bin/sh
# Runs the unit tests, then every fixture in tests/ on all three kanso
# engines: the interpreter (`kanso run --interp`), a dev build and a release
# build. Each run's stdout, stderr and exit status are rendered into one text
# and compared with tests/<name>.expected and with the other two engines.
#
# The expected files were produced by this port and checked against the
# reference jlox from Crafting Interpreters; they differ from jlox only in
# error_stack_overflow (see README.md).
#
#   sh check.sh            check everything
#   KANSO=/path/to/kanso   use another compiler
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failed=0

echo "== unit tests"
if ! "$KANSO" test lox > "$tmp/unit" 2>&1; then
  cat "$tmp/unit"
  failed=1
fi
tail -1 "$tmp/unit"

echo "== build"
"$KANSO" build main.kso > /dev/null || exit 1
mv main "$tmp/lox-dev"
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/lox-release"
rm -f main.ll

# stdout, then stderr if there was any, then the exit status. The streams are
# captured apart because the engines interleave them differently when they
# share a pipe (FRICTION F25).
render() {
  "$@" > "$tmp/out" 2> "$tmp/err"
  code=$?
  cat "$tmp/out"
  if [ -s "$tmp/err" ]; then
    echo "--- stderr"
    cat "$tmp/err"
  fi
  echo "--- exit $code"
}

echo "== fixtures"
count=0
for lox in tests/*.lox; do
  name=$(basename "$lox" .lox)
  expected="tests/$name.expected"
  count=$((count + 1))
  render "$KANSO" run main.kso --interp -- "$lox" > "$tmp/interp"
  render "$tmp/lox-dev" "$lox" > "$tmp/dev"
  render "$tmp/lox-release" "$lox" > "$tmp/release"
  bad=""
  for engine in interp dev release; do
    if [ ! -f "$expected" ] || ! cmp -s "$tmp/$engine" "$expected"; then
      bad="$bad $engine"
    fi
  done
  if [ -n "$bad" ]; then
    echo "FAIL $name:$bad"
    if [ -f "$expected" ]; then
      for engine in $bad; do
        diff "$expected" "$tmp/$engine" | head -20
      done
    else
      echo "  missing $expected"
    fi
    failed=1
  else
    echo "ok   $name"
  fi
done

if [ "$failed" -ne 0 ]; then
  echo "FAILED"
  exit 1
fi
echo "all $count fixtures agree on all three engines"
