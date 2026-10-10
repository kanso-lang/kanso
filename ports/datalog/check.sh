#!/bin/sh
# Runs the unit tests, then every fixture on all three kanso engines: the
# interpreter (`kanso run --interp`), a dev build and a release build. Each
# run's stdout, then its stderr if there was any, then its exit status, is
# compared with tests/<name>.expected and with the other two engines.
#
# A fixture is tests/<name>.dl, run as `datalog tests/<name>.dl`. Two more
# cases exercise the command line: `missing_file` names a file that does not
# exist, and `stdin` feeds tests/tc_small.dl on standard input.
#
#   sh check.sh             check everything
#   sh check.sh --update    rewrite the expected files from the interpreter
#   KANSO=/path/to/kanso    use another compiler
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
update=0
[ "${1:-}" = "--update" ] && update=1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failed=0

echo "== unit tests"
if ! "$KANSO" test datalog > "$tmp/unit" 2>&1; then
  cat "$tmp/unit"
  failed=1
fi
tail -1 "$tmp/unit"

echo "== build"
"$KANSO" build main.kso > /dev/null || exit 1
mv main "$tmp/datalog-dev"
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/datalog-release"
rm -f main.ll

# stdout, then stderr if there was any, then the exit status
render() {
  input=$1
  shift
  "$@" < "$input" > "$tmp/out" 2> "$tmp/err"
  code=$?
  cat "$tmp/out"
  if [ -s "$tmp/err" ]; then
    echo "--- stderr"
    cat "$tmp/err"
  fi
  echo "--- exit $code"
}

# one case: its name, the file fed to stdin, then the program's arguments
run_case() {
  name=$1
  input=$2
  shift 2
  expected="tests/$name.expected"
  render "$input" "$KANSO" run main.kso --interp -- "$@" > "$tmp/interp"
  render "$input" "$tmp/datalog-dev" "$@" > "$tmp/dev"
  render "$input" "$tmp/datalog-release" "$@" > "$tmp/release"
  if [ "$update" -eq 1 ]; then
    cp "$tmp/interp" "$expected"
  fi
  bad=""
  for engine in interp dev release; do
    if [ ! -f "$expected" ] || ! cmp -s "$tmp/$engine" "$expected"; then
      bad="$bad $engine"
    fi
  done
  count=$((count + 1))
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
}

echo "== fixtures"
count=0
for dl in tests/*.dl; do
  run_case "$(basename "$dl" .dl)" /dev/null "$dl"
done
run_case missing_file /dev/null tests/does_not_exist.dl
run_case stdin tests/tc_small.dl

if [ "$failed" -ne 0 ]; then
  echo "FAILED"
  exit 1
fi
echo "all $count fixtures agree on all three engines"
