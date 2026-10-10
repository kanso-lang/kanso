#!/bin/sh
# Runs the unit tests, checks that the embedded math library matches
# lib/math.bc, then runs every fixture in tests/ on all three kanso engines:
# the interpreter (`kanso run --interp`), a dev build and a release build.
# Each run's stdout, stderr and exit status are rendered into one text and
# compared with tests/NAME.expected and with the other two engines.
#
# A fixture is tests/NAME.bc. tests/NAME.args holds its arguments, with @
# standing for the fixture (without an @ they go before it); tests/NAME.env
# holds environment settings; tests/NAME.stdin is its standard input, which
# is otherwise empty.
#
# The expected files agree with GNU bc 1.07.1 except where README.md lists a
# divergence; compare_gnu.sh shows the differences when bc is installed.
#
#   sh check.sh            check everything
#   sh check.sh --update   rewrite the expected files from the release build
#   KANSO=/path/to/kanso   use another compiler
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failed=0

echo "== unit tests"
if ! "$KANSO" test bc > "$tmp/unit" 2>&1; then
  cat "$tmp/unit"
  failed=1
fi
tail -1 "$tmp/unit"

echo "== embedded library"
cp bc/mathlib.kso "$tmp/mathlib.kso"
sh embed_mathlib.sh
if cmp -s bc/mathlib.kso "$tmp/mathlib.kso"; then
  echo "bc/mathlib.kso matches lib/math.bc"
else
  echo "bc/mathlib.kso was stale; regenerated it from lib/math.bc"
  failed=1
fi

echo "== build"
"$KANSO" build main.kso > /dev/null || exit 1
mv main "$tmp/bc-dev"
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/bc-release"
rm -f main.ll

# stdout, then stderr if there was any, then the exit status. The streams are
# captured apart so that the engines' buffering cannot interleave them
# differently.
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

set_case() {
  args="@"
  [ -f "tests/$1.args" ] && args=$(cat "tests/$1.args")
  case "$args" in *@*) ;; *) args="$args @" ;; esac
  args=$(echo "$args" | sed "s|@|tests/$1.bc|")
  vars=""
  [ -f "tests/$1.env" ] && vars=$(cat "tests/$1.env")
  input=/dev/null
  [ -f "tests/$1.stdin" ] && input="tests/$1.stdin"
}

echo "== fixtures"
count=0
for fixture in tests/*.bc; do
  name=$(basename "$fixture" .bc)
  expected="tests/$name.expected"
  count=$((count + 1))
  set_case "$name"
  # shellcheck disable=SC2086
  render env $vars "$KANSO" run main.kso --interp -- $args < "$input" \
    > "$tmp/interp"
  # shellcheck disable=SC2086
  render env $vars "$tmp/bc-dev" $args < "$input" > "$tmp/dev"
  # shellcheck disable=SC2086
  render env $vars "$tmp/bc-release" $args < "$input" > "$tmp/release"
  if [ "${1:-}" = "--update" ]; then
    cp "$tmp/release" "$expected"
  fi
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
