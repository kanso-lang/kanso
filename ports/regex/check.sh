#!/bin/sh
# Runs the unit tests, then every fixture in tests/ on all three kanso
# engines: the interpreter (`kanso run --interp`), a dev build and a release
# build. A fixture is tests/NAME.args, one command-line argument per line,
# with tests/NAME.stdin fed to standard input when it exists. Each run's
# stdout, stderr and exit status are rendered into one text and compared
# with tests/NAME.expected and with the other two engines.
#
#   sh check.sh              check everything
#   sh check.sh --oracle     also diff the --table fixtures against Go's
#                            regexp (needs `go`) and show where Python's re
#                            differs (needs python3)
#   KANSO=/path/to/kanso     use another compiler
set -u
mode=${1:-}
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
failed=0

echo "== unit tests"
for module in re grep; do
  if ! "$KANSO" test "$module" > "$tmp/unit" 2>&1; then
    cat "$tmp/unit"
    failed=1
  fi
  echo "$module: $(tail -1 "$tmp/unit")"
done

echo "== build"
"$KANSO" build main.kso > /dev/null || exit 1
mv main "$tmp/regex-dev"
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/regex-release"
rm -f main.ll

# stdout, then stderr if there was any, then the exit status.
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

echo "== fixtures"
count=0
for args in tests/*.args; do
  name=$(basename "$args" .args)
  expected="tests/$name.expected"
  input="tests/$name.stdin"
  [ -f "$input" ] || input=/dev/null
  count=$((count + 1))
  set --
  while IFS= read -r word; do
    set -- "$@" "$word"
  done < "$args"
  render "$input" "$KANSO" run main.kso --interp -- "$@" > "$tmp/interp"
  render "$input" "$tmp/regex-dev" "$@" > "$tmp/dev"
  render "$input" "$tmp/regex-release" "$@" > "$tmp/release"
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

if [ "$mode" = "--oracle" ]; then
  echo "== oracles"
  for table in table_basic table_errors table_longest; do
    flag=""
    [ "$table" = table_longest ] && flag=-L
    "$tmp/regex-release" $flag --table "tests/$table.tsv" > "$tmp/mine"
    if go run oracle/table.go $flag < "tests/$table.tsv" > "$tmp/go" &&
      cmp -s "$tmp/go" "$tmp/mine"; then
      echo "ok   $table agrees with Go's regexp"
    else
      echo "FAIL $table differs from Go's regexp"
      diff "$tmp/go" "$tmp/mine" | head -20
      failed=1
    fi
  done
  python3 -I oracle/table.py < tests/table_basic.tsv > "$tmp/py" 2> /dev/null
  "$tmp/regex-release" --table tests/table_basic.tsv > "$tmp/mine"
  differ=$(diff "$tmp/py" "$tmp/mine" | grep '^<' | grep -vc 'python error$')
  echo "     table_basic: $differ cases where Python's re matches differently"
fi

if [ "$failed" -ne 0 ]; then
  echo "FAILED"
  exit 1
fi
echo "all $count fixtures agree on all three engines"
