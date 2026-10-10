#!/bin/sh
# Runs every fixture through GNU bc and through this port's release build,
# and prints the differences. GNU's `adr=N` is the address of a bytecode
# instruction, which this port does not have, so it is rewritten to the
# `adr=0` the port prints before comparing. The fixtures listed in
# README.md under "Divergences" are expected to differ, and how.
#
#   sh compare_gnu.sh            every fixture
#   sh compare_gnu.sh name ...   only these
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
command -v bc > /dev/null || { echo "GNU bc is not installed"; exit 1; }
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/port"
rm -f main.ll

render() {
  "$@" > "$tmp/out" 2> "$tmp/err"
  code=$?
  cat "$tmp/out"
  if [ -s "$tmp/err" ]; then
    echo "--- stderr"
    sed 's/adr=[0-9]*/adr=0/' "$tmp/err"
  fi
  echo "--- exit $code"
}

# A fixture's arguments are tests/NAME.args with @ standing for the fixture
# (without an @ they go before it), its environment is tests/NAME.env, and
# its standard input tests/NAME.stdin.
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

if [ $# -gt 0 ]; then names="$*"; else
  names=$(for f in tests/*.bc; do basename "$f" .bc; done)
fi
same=0
differ=0
for name in $names; do
  set_case "$name"
  # shellcheck disable=SC2086
  render env $vars bc -q $args < "$input" > "$tmp/gnu"
  # shellcheck disable=SC2086
  render env $vars "$tmp/port" $args < "$input" > "$tmp/mine"
  if cmp -s "$tmp/gnu" "$tmp/mine"; then
    same=$((same + 1))
  else
    differ=$((differ + 1))
    echo "== $name differs from GNU bc (< GNU, > port)"
    diff "$tmp/gnu" "$tmp/mine"
  fi
done
echo "$same fixtures match GNU bc, $differ differ"
