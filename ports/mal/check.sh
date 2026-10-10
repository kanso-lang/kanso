#!/bin/sh
# Runs every fixture on all three engines and compares each output with the
# expected file. The engines are the interpreter (`kanso run --interp`, the
# oracle), a dev-tier native build and a release build. A difference from the
# expected file, or between engines, fails the run.
#
#   KANSO=/path/to/kanso sh check.sh          everything
#   KANSO=... sh check.sh --update            rewrite expected files from the
#                                             interpreter, then check
#
# Fixture kinds:
#   fixtures/repl/NAME.in        piped into the REPL           -> NAME.out
#   fixtures/run/NAME.mal        run as a file (NAME.args)     -> NAME.out
#   fixtures/selfhost/NAME.in    piped into mal-in-mal         -> NAME.out
#   tests/STEP.mal               runtest cases, direct         -> tests/expected/STEP.out
#   tests/STEP.mal               the same, through mal-in-mal  -> tests/expected/STEP.hosted.out
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")" || exit 2
UPDATE=${1:-}
SELF=examples/self_host/stepA_mal.mal

# KANSO_NOW pins time-ms, so a run is the same run every time.
KANSO_NOW=1700000000000
export KANSO_NOW

fail=0
cases=0

echo "kanso test"
if ! "$KANSO" test lisp; then
  echo "FAIL: kanso test"
  fail=1
fi

mkdir -p build
echo "building the dev and release binaries"
"$KANSO" build . >/dev/null || { echo "FAIL: kanso build"; exit 1; }
mv mal build/mal-dev
"$KANSO" build . --release >/dev/null || { echo "FAIL: kanso build --release"; exit 1; }
mv mal build/mal-release
rm -f mal.ll

# run ENGINE STDIN ARGS...: one program run, stdout and the exit status.
run() {
  engine=$1
  input=$2
  shift 2
  case $engine in
    interp) "$KANSO" run . --interp -- "$@" <"$input" 2>/dev/null ;;
    dev) build/mal-dev "$@" <"$input" 2>/dev/null ;;
    release) build/mal-release "$@" <"$input" 2>/dev/null ;;
  esac
  echo "[exit $?]"
}

# check NAME EXPECTED STDIN ARGS...: run on all three engines and compare.
check() {
  name=$1
  expected=$2
  input=$3
  shift 3
  cases=$((cases + 1))
  run interp "$input" "$@" >build/got.interp
  run dev "$input" "$@" >build/got.dev
  run release "$input" "$@" >build/got.release
  if [ "$UPDATE" = "--update" ]; then
    cp build/got.interp "$expected"
  fi
  ok=1
  for engine in interp dev release; do
    if ! cmp -s "$expected" "build/got.$engine"; then
      echo "FAIL: $name ($engine differs from $expected)"
      diff "$expected" "build/got.$engine" | head -10
      ok=0
    fi
  done
  if ! cmp -s build/got.interp build/got.dev || ! cmp -s build/got.dev build/got.release; then
    echo "FAIL: $name (the engines disagree with each other)"
    ok=0
  fi
  if [ $ok = 1 ]; then
    echo "ok: $name"
  else
    fail=1
  fi
}

for f in fixtures/repl/*.in; do
  check "repl $(basename "$f" .in)" "${f%.in}.out" "$f"
done

for f in fixtures/run/*.mal; do
  args=""
  if [ -f "${f%.mal}.args" ]; then
    args=$(cat "${f%.mal}.args")
  fi
  # shellcheck disable=SC2086
  check "run $(basename "$f" .mal)" "${f%.mal}.out" /dev/null "$f" $args
done

for f in fixtures/selfhost/*.in; do
  check "selfhost $(basename "$f" .in)" "${f%.in}.out" "$f" "$SELF"
done

mkdir -p tests/expected
for f in tests/step*.mal; do
  step=$(basename "$f" .mal)
  check "runtest $step" "tests/expected/$step.out" /dev/null --test "$f"
done

# mal-in-mal has no tail calls of its own, so step 5's ten-thousand-deep
# loops are left out here as they are upstream; step 1 only reads and prints.
for f in tests/step2*.mal tests/step3*.mal tests/step4*.mal tests/step[6-9]*.mal tests/stepA*.mal; do
  step=$(basename "$f" .mal)
  check "runtest $step via mal-in-mal" "tests/expected/$step.hosted.out" /dev/null --test "$f" --via "$SELF"
done

rm -f build/got.interp build/got.dev build/got.release
echo
if [ $fail = 0 ]; then
  echo "all $cases fixtures agree on interp, dev and release"
else
  echo "FAILED"
fi
exit $fail
