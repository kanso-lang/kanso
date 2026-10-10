#!/bin/sh
# Runs every fixture on all three engines -- the interpreter, a dev build and a
# release build -- and fails if any output differs from its expected file or
# from another engine's.
#
#   tests/valid/NAME.toml     decodes to tests/valid/NAME.json
#   tests/invalid/NAME.toml   is refused with tests/invalid/NAME.err, exit 1
#   tests/encode/NAME.json    encodes to tests/encode/NAME.toml, or is refused
#                             with tests/encode/NAME.err, exit 1
#   tests/get/NAME.query      a document (first line, relative to tests/) and a
#                             path (second line); `toml get` prints NAME.out,
#                             which ends with the exit line
#
# Every valid document also makes a round trip on every engine: decode, encode
# the JSON back to TOML, decode that, and the JSON must not change.
#
# KANSO points at the compiler. CHECK_WRITE=1 rewrites the expected files from
# the interpreter's output instead of comparing against them.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failures=0
cases=0

fail() {
  echo "FAIL: $*"
  failures=$((failures + 1))
}

echo "== unit tests"
if ! "$KANSO" test "$here/toml" > "$work/unit.out" 2>&1; then
  cat "$work/unit.out"
  fail "kanso test toml"
fi
tail -1 "$work/unit.out"

echo "== build"
mkdir -p "$work/dev" "$work/release"
(cd "$work/dev" && "$KANSO" build "$here/main.kso" > /dev/null) || {
  echo "dev build failed"; exit 1; }
(cd "$work/release" && "$KANSO" build "$here/main.kso" --release > /dev/null) || {
  echo "release build failed"; exit 1; }

engines="interp dev release"

# run ENGINE ARGS... : the program's stdout and stderr together, then a final
# line naming its exit status.
run() {
  engine=$1
  shift
  case $engine in
    interp) "$KANSO" run "$here/main.kso" --interp -- "$@" 2>&1 ;;
    dev) "$work/dev/main" "$@" 2>&1 ;;
    release) "$work/release/main" "$@" 2>&1 ;;
  esac
  echo "[exit $?]"
}

# check NAME EXPECTED ARGS... : runs ARGS on every engine and compares each
# output with EXPECTED, which holds the output followed by its exit line.
check() {
  name=$1
  expected=$2
  shift 2
  cases=$((cases + 1))
  for engine in $engines; do
    run "$engine" "$@" > "$work/$engine.out"
  done
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    cp "$work/interp.out" "$work/expected.out"
  else
    cp "$expected" "$work/expected.out"
  fi
  for engine in $engines; do
    if ! cmp -s "$work/expected.out" "$work/$engine.out"; then
      fail "$name on $engine"
      diff "$work/expected.out" "$work/$engine.out" | head -20
    fi
  done
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    cp "$work/interp.out" "$expected"
  fi
}

# The expected files hold just the output; the exit line is added here.
expect() {
  cat "$1"
  echo "[exit $2]"
}

echo "== valid documents"
for toml in "$here"/tests/valid/*.toml; do
  base=${toml%.toml}
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    run interp decode "$toml" | sed '$d' > "$base.json"
  fi
  expect "$base.json" 0 > "$work/want"
  check "valid/$(basename "$toml")" "$work/want" decode "$toml"
done

echo "== invalid documents"
for toml in "$here"/tests/invalid/*.toml; do
  base=${toml%.toml}
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    run interp decode "$toml" | sed '$d' > "$base.err"
  fi
  expect "$base.err" 1 > "$work/want"
  check "invalid/$(basename "$toml")" "$work/want" decode "$toml"
done

echo "== encoding"
for json in "$here"/tests/encode/*.json; do
  base=${json%.json}
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    rm -f "$base.err" "$base.toml"
    run interp encode "$json" > "$work/written"
    if tail -1 "$work/written" | grep -q 'exit 0'; then
      sed '$d' "$work/written" > "$base.toml"
    else
      sed '$d' "$work/written" > "$base.err"
    fi
  fi
  if [ -f "$base.err" ]; then
    expect "$base.err" 1 > "$work/want"
  else
    expect "$base.toml" 0 > "$work/want"
  fi
  check "encode/$(basename "$json")" "$work/want" encode "$json"
done

echo "== lookups"
for query in "$here"/tests/get/*.query; do
  base=${query%.query}
  doc="$here/tests/$(sed -n 1p "$query")"
  path=$(sed -n 2p "$query")
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    run interp get "$doc" "$path" > "$base.out"
  fi
  check "get/$(basename "$query")" "$base.out" get "$doc" "$path"
done

echo "== round trips"
for toml in "$here"/tests/valid/*.toml; do
  name=$(basename "$toml" .toml)
  cases=$((cases + 1))
  for engine in $engines; do
    run "$engine" decode "$toml" | sed '$d' > "$work/first.json"
    run "$engine" encode "$work/first.json" | sed '$d' > "$work/again.toml"
    run "$engine" decode "$work/again.toml" | sed '$d' > "$work/second.json"
    if ! cmp -s "$work/first.json" "$work/second.json"; then
      fail "round trip of $name on $engine"
      diff "$work/first.json" "$work/second.json" | head -20
    fi
  done
done

echo "== $cases cases, $failures failures, engines: $engines"
[ "$failures" -eq 0 ]
