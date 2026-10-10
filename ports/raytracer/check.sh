#!/bin/sh
# The port's whole check: every module's unit tests, then every fixture run
# three ways -- on the interpreter, as a dev-tier binary and as a release
# binary. Each run's stdout (the image, or nothing) and its stderr with the
# exit status appended are compared with the expected files and with the
# other two engines.
#
#   sh check.sh             run everything
#   sh check.sh --update    rewrite the expected files from the interpreter
#
# A fixture is fixtures/<name>.args, one line of arguments; its expected
# output is fixtures/<name>.stdout and fixtures/<name>.stderr.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
update=${1:-}
fail=0
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

echo "== unit tests"
for module in vec fmath rng tracer scenefile cli; do
  if "$KANSO" test "$module" > "$out/test.txt" 2>&1; then
    tail -n 1 "$out/test.txt" | sed "s/^/$module: /"
  else
    cat "$out/test.txt"
    echo "FAIL: unit tests in $module"
    fail=1
  fi
done

echo "== build"
mkdir -p build/dev build/release
rm -f build/dev/main build/release/main
(cd build/dev && "$KANSO" build ../../main.kso > /dev/null) || {
  echo "FAIL: dev build"; exit 1; }
(cd build/release && "$KANSO" build ../../main.kso --release > /dev/null) || {
  echo "FAIL: release build"; exit 1; }

# run <engine> <name>: one engine on one fixture, into $out/<name>.<engine>.*
run() {
  args=$(cat "fixtures/$2.args")
  case $1 in
    interp)  set -- "$1" "$2" "$KANSO" run main.kso --interp -- ;;
    dev)     set -- "$1" "$2" build/dev/main ;;
    release) set -- "$1" "$2" build/release/main ;;
  esac
  engine=$1 name=$2
  shift 2
  # $args is split on spaces on purpose: it is a list of words.
  # shellcheck disable=SC2086
  "$@" $args > "$out/$name.$engine.stdout" 2> "$out/$name.$engine.stderr"
  echo "exit $?" >> "$out/$name.$engine.stderr"
}

echo "== fixtures"
for argsfile in fixtures/*.args; do
  name=$(basename "$argsfile" .args)
  start=$(date +%s)
  for engine in interp dev release; do
    run "$engine" "$name"
  done
  secs=$(( $(date +%s) - start ))
  if [ "$update" = "--update" ]; then
    cp "$out/$name.interp.stdout" "fixtures/$name.stdout"
    cp "$out/$name.interp.stderr" "fixtures/$name.stderr"
  fi
  verdict=ok
  for engine in interp dev release; do
    for stream in stdout stderr; do
      if ! cmp -s "$out/$name.$engine.$stream" "fixtures/$name.$stream"; then
        echo "FAIL: $name: $engine $stream differs from the expected file"
        verdict=FAIL
        fail=1
      fi
    done
  done
  for engine in dev release; do
    for stream in stdout stderr; do
      if ! cmp -s "$out/$name.$engine.$stream" "$out/$name.interp.$stream"; then
        echo "FAIL: $name: $engine $stream differs from the interpreter"
        verdict=FAIL
        fail=1
      fi
    done
  done
  echo "$name: $verdict (${secs}s for three engines)"
done

# scenes/three.scene describes chapter 11's scene in the file format, so its
# image must be the built-in scene's image, byte for byte.
if cmp -s fixtures/file_three.stdout fixtures/glass.stdout; then
  echo "file_three matches glass: ok"
else
  echo "FAIL: scenes/three.scene renders differently from the glass scene"
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "check: FAILED"
  exit 1
fi
echo "check: all engines agree"
