#!/bin/sh
# The port's test suite. It runs the unit tests, then every fixture in
# fixtures/cases.txt three ways -- on the interpreter, as a dev-tier native
# binary, and as a release binary -- and fails if any of the three differs
# from the expected file or from the others.
#
# KANSO names the compiler; it defaults to the build the brief points at.
set -u
K=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
fail=0

echo "== unit tests"
if ! "$K" test "$here/search" >"$work/unit" 2>&1; then
  cat "$work/unit"
  fail=1
fi
tail -1 "$work/unit"

echo "== builds"
mkdir -p "$work/dev" "$work/release"
(cd "$work/dev" && "$K" build "$here/main.kso" >/dev/null) || fail=1
(cd "$work/release" && "$K" build "$here/main.kso" --release >/dev/null) \
  || fail=1
[ -x "$work/dev/main" ] && [ -x "$work/release/main" ] || {
  echo "a build failed"; exit 1; }

sh "$here/fixtures/make_tree.sh" "$work/tree"

# Run one engine on one case, from the directory holding the tree (or from
# the directory a leading `@dir` names), and write stdout, stderr and the
# exit status into one file.
run() {
  out=$1; shift
  (
    cd "$work/$where" || exit 1
    "$@" >"$work/stdout" 2>"$work/stderr" </dev/null
    echo $? >"$work/code"
  )
  { cat "$work/stdout"; echo "[stderr]"; cat "$work/stderr"
    echo "[exit $(cat "$work/code")]"; } >"$out"
}

echo "== fixtures"
cases=0
while IFS='	' read -r name args; do
  case "$name" in ''|'#'*) continue ;; esac
  cases=$((cases + 1))
  want="$here/fixtures/expected/$name.out"
  where=.
  case "$args" in @*) where=${args%% *}; where=${where#@}; args=${args#* } ;; esac
  eval "set -- $args"
  run "$work/$name.interp" "$K" run "$here/main.kso" --interp -- "$@"
  run "$work/$name.dev" "$work/dev/main" "$@"
  run "$work/$name.release" "$work/release/main" "$@"
  for engine in interp dev release; do
    if ! cmp -s "$want" "$work/$name.$engine"; then
      echo "FAIL $name ($engine)"
      diff "$want" "$work/$name.$engine" | head -20
      fail=1
    fi
  done
done <"$here/fixtures/cases.txt"
echo "$cases fixtures, 3 engines each"

if [ "$fail" -eq 0 ]; then echo "all green"; else echo "FAILED"; fi
exit "$fail"
