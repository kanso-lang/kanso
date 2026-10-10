#!/bin/sh
# Runs every fixture three ways -- on the interpreter (`kanso run --interp`),
# on a dev build and on a release build -- and fails if any engine's output
# differs from the expected file or from another engine's.
#
# A case is a command line plus up to three expected files:
#   <expected>          stdout, byte for byte (absent means empty)
#   <expected>.err      stderr (absent means empty)
#   <expected>.status   exit status (absent means 0)
#
# Usage: sh check.sh            check everything
#        sh check.sh --bless    rewrite the expected files from the
#                               interpreter's output (review the diff!)
#
# KANSO points at the compiler; the default is the toolchain the port brief
# names.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
cd "$(dirname "$0")" || exit 1
root=$(pwd)
bless=no
[ "${1:-}" = --bless ] && bless=yes

out=$root/build/out
rm -rf "$out"
mkdir -p "$out" build/dev build/release

echo "== unit tests (interpreter) =="
"$KANSO" test mustache || exit 1

echo "== build =="
# The binary is named for the directory, and the engine module has the same
# name, so the build runs from a directory where the name is free.
(cd build/dev && "$KANSO" build "$root" >/dev/null) || { echo "dev build failed"; exit 1; }
(cd build/release && "$KANSO" build "$root" --release >/dev/null) \
  || { echo "release build failed"; exit 1; }

run_engine() {
  engine=$1; shift
  case $engine in
    interp) "$KANSO" run "$root" --interp -- "$@" ;;
    dev) "$root/build/dev/mustache" "$@" ;;
    release) "$root/build/release/mustache" "$@" ;;
  esac
}

cases=0
failed=0

# check_case <name> <expected-file> <args...>
check_case() {
  name=$1; expected=$2; shift 2
  cases=$((cases + 1))
  bad=""
  for engine in interp dev release; do
    dir=$out/$cases.$engine
    mkdir -p "$dir"
    run_engine "$engine" "$@" >"$dir/stdout" 2>"$dir/stderr"
    echo $? >"$dir/status"
  done
  if [ "$bless" = yes ]; then
    i=$out/$cases.interp
    cp "$i/stdout" "$expected"
    if [ -s "$i/stderr" ]; then cp "$i/stderr" "$expected.err"; else rm -f "$expected.err"; fi
    if [ "$(cat "$i/status")" != 0 ]; then cp "$i/status" "$expected.status"; else rm -f "$expected.status"; fi
  fi
  : >"$out/empty"
  echo 0 >"$out/zero"
  want_out=$expected; [ -f "$want_out" ] || want_out=$out/empty
  want_err=$expected.err; [ -f "$want_err" ] || want_err=$out/empty
  want_status=$expected.status; [ -f "$want_status" ] || want_status=$out/zero
  for engine in interp dev release; do
    dir=$out/$cases.$engine
    cmp -s "$dir/stdout" "$want_out" || bad="$bad $engine:stdout"
    cmp -s "$dir/stderr" "$want_err" || bad="$bad $engine:stderr"
    cmp -s "$dir/status" "$want_status" || bad="$bad $engine:status"
  done
  for part in stdout stderr status; do
    cmp -s "$out/$cases.interp/$part" "$out/$cases.dev/$part" || bad="$bad interp/dev:$part"
    cmp -s "$out/$cases.interp/$part" "$out/$cases.release/$part" || bad="$bad interp/release:$part"
  done
  if [ -n "$bad" ]; then
    failed=$((failed + 1))
    echo "FAIL $name:$bad"
    echo "  outputs kept in $out/$cases.*"
  else
    echo "ok   $name"
  fi
}

echo "== spec suites =="
for suite in fixtures/spec/*.json; do
  base=${suite%.json}
  check_case "spec $(basename "$base")" "$base.expected" spec "$suite"
done

echo "== renders =="
for dir in fixtures/render/*/ fixtures/errors/*/; do
  dir=${dir%/}
  if [ -f "$dir/partials.json" ]; then
    check_case "render $dir" "$dir/expected" render \
      "$dir/template.mustache" "$dir/data.json" "$dir/partials.json"
  else
    check_case "render $dir" "$dir/expected" render \
      "$dir/template.mustache" "$dir/data.json"
  fi
done

echo "== tokens =="
for template in fixtures/tokens/*.mustache; do
  base=${template%.mustache}
  check_case "tokens $(basename "$base")" "$base.expected" tokens "$template"
done

echo "== options =="
o=fixtures/options
check_case "letter" $o/letter.plain render $o/letter.mustache $o/letter.json
check_case "letter --no-escape" $o/letter.raw \
  render --no-escape $o/letter.mustache $o/letter.json
check_case "letter --strict" $o/letter.strict \
  render --strict $o/letter.mustache $o/letter.json
check_case "sections --strict" $o/sections.strict \
  render $o/sections.mustache $o/sections.json --strict
check_case "unknown option" $o/bogus render --bogus $o/letter.mustache $o/letter.json

echo "== command line =="
check_case "no arguments" fixtures/cli/no-args
check_case "unknown command" fixtures/cli/unknown frobnicate
check_case "missing template" fixtures/cli/missing-template render \
  fixtures/cli/nowhere.mustache fixtures/render/page/data.json

echo "$cases cases, $failed failed, three engines each"
[ "$failed" = 0 ]
