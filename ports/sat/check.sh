#!/bin/sh
# Runs every fixture on all three engines -- the interpreter, a dev build and
# a release build -- and fails if any output differs from its expected file
# or from another engine's. Output here means stdout and stderr together,
# followed by a line naming the exit status.
#
#   tests/solve/NAME.cnf     solved; the answer is in NAME.out
#   tests/errors/NAME.cnf    malformed input; the refusal is in NAME.out
#   tests/cli/NAME.args      one command line (NAME.stdin feeds stdin);
#                            the result is in NAME.out
#   tests/bench/MANIFEST     the benchmark set: each NAME.cnf must be what
#                            `gen` prints for its arguments on every engine,
#                            and solving it must print NAME.out
#
# Then, when python3 is present, tests/oracle.py -- a separate DPLL sharing
# no code with the solver -- checks the satisfiability verdict and the model
# of every fixture marked for it, and of a batch of random formulas solved
# by the release build.
#
# KANSO points at the compiler. CHECK_WRITE=1 rewrites the expected files
# from the interpreter's output instead of comparing against them.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failures=0
cases=0

fail() {
  echo "FAIL: $*"
  failures=$((failures + 1))
}

echo "== unit tests"
for module in dimacs solver gen cli; do
  if ! "$KANSO" test "$module" > "$work/unit.out" 2>&1; then
    cat "$work/unit.out"
    fail "kanso test $module"
  fi
  echo "$module: $(tail -1 "$work/unit.out")"
done

echo "== build"
mkdir -p "$work/dev" "$work/release"
(cd "$work/dev" && "$KANSO" build "$here/main.kso" > /dev/null) || {
  echo "dev build failed"; exit 1; }
(cd "$work/release" && "$KANSO" build "$here/main.kso" --release > /dev/null) || {
  echo "release build failed"; exit 1; }

# run ENGINE STDIN ARGS... : the program's stdout and stderr together, then
# a final line naming its exit status
run() {
  engine=$1
  input=$2
  shift 2
  case $engine in
    interp) "$KANSO" run "$here/main.kso" --interp -- "$@" < "$input" 2>&1 ;;
    dev) "$work/dev/main" "$@" < "$input" 2>&1 ;;
    release) "$work/release/main" "$@" < "$input" 2>&1 ;;
  esac
  echo "[exit $?]"
}

# check NAME EXPECTED STDIN ARGS... : runs ARGS on every engine and compares
# each output with EXPECTED
check() {
  name=$1
  expected=$2
  input=$3
  shift 3
  cases=$((cases + 1))
  for engine in interp dev release; do
    run "$engine" "$input" "$@" > "$work/$engine.out"
  done
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    cp "$work/interp.out" "$expected"
  fi
  for engine in interp dev release; do
    if ! cmp -s "$work/$engine.out" "$expected"; then
      fail "$name on $engine"
      diff "$expected" "$work/$engine.out" | head -10
    fi
  done
}

echo "== solve"
for f in tests/solve/*.cnf; do
  check "$f" "${f%.cnf}.out" /dev/null solve "$f"
done

echo "== malformed input"
for f in tests/errors/*.cnf; do
  check "$f" "${f%.cnf}.out" /dev/null solve "$f"
done

echo "== command line"
for f in tests/cli/*.args; do
  base=${f%.args}
  input=/dev/null
  [ -f "$base.stdin" ] && input="$base.stdin"
  # shellcheck disable=SC2046
  check "$f" "$base.out" "$input" $(cat "$f")
done

echo "== benchmarks"
oracle_list="$work/oracle.list"
: > "$oracle_list"
grep -v '^#' tests/bench/MANIFEST > "$work/manifest"
while read -r line; do
  bench=$(echo "$line" | awk '{print $1}')
  oracle=$(echo "$line" | awk '{print $NF}')
  flags=$(echo "$line" | awk '{print $(NF-1)}')
  genargs=$(echo "$line" | awk '{for (i = 2; i < NF - 1; i++) printf "%s ", $i}')
  cnf="tests/bench/$bench.cnf"
  # the formula on disk, as `gen` must print it and exit
  if [ "${CHECK_WRITE:-}" = 1 ]; then
    # shellcheck disable=SC2086
    run interp /dev/null gen $genargs | sed '$d' > "$cnf"
  fi
  { cat "$cnf"; echo "[exit 0]"; } > "$work/generated"
  # shellcheck disable=SC2086
  check "gen $genargs" "$work/generated" /dev/null gen $genargs
  start=$(date +%s)
  check "$cnf" "tests/bench/$bench.out" /dev/null solve "$flags" "$cnf"
  took=$(($(date +%s) - start))
  verdict=$(grep '^s ' "tests/bench/$bench.out")
  echo "$bench: $verdict, three engines in $took s"
  [ "$oracle" = yes ] && echo "$cnf tests/bench/$bench.out" >> "$oracle_list"
done < "$work/manifest"

echo "== oracle"
if command -v python3 > /dev/null; then
  for f in tests/solve/*.cnf; do
    echo "$f ${f%.cnf}.out" >> "$oracle_list"
  done
  checked=0
  while read -r cnf out; do
    sed '$d' "$out" > "$work/answer.txt"
    if ! python3 -I tests/oracle.py "$cnf" "$work/answer.txt"; then
      fail "oracle disagrees on $cnf"
    fi
    checked=$((checked + 1))
  done < "$oracle_list"
  fuzzed=0
  for seed in $(seq 1 60); do
    vars=$((20 + seed % 31))
    count=$((vars * 43 / 10))
    "$work/release/main" gen random "$vars" "$count" "$seed" > "$work/f.cnf"
    flag=--luby
    [ $((seed % 2)) = 0 ] && flag=--geometric
    "$work/release/main" solve "$flag" "$work/f.cnf" > "$work/answer.txt"
    if ! python3 -I tests/oracle.py "$work/f.cnf" "$work/answer.txt"; then
      fail "oracle disagrees on gen random $vars $count $seed ($flag)"
    fi
    fuzzed=$((fuzzed + 1))
  done
  echo "oracle agreed on $checked fixtures and $fuzzed random formulas"
else
  echo "python3 is not installed: the oracle did not run"
fi

echo
echo "$cases cases on three engines, $failures failures"
[ "$failures" = 0 ]
