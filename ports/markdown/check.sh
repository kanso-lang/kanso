#!/bin/sh
# Run the unit tests, then render every fixture three ways: on the
# interpreter, as a dev build and as a release build. Each output must match
# the fixture's expected HTML byte for byte, and so must agree with the
# other two. Exits non-zero on the first kind of failure it finds, after
# listing every fixture that failed.
#
# A fixture may carry a `.args` file of command-line arguments, given to all
# three engines; paths in it are relative to this directory. Standard error
# is compared along with standard output.
#
# KANSO names the compiler; it defaults to the toolchain on main.
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

echo "== unit tests"
"$KANSO" test "$here/commonmark" > "$work/test.out" 2>&1
status=$?
tail -1 "$work/test.out"
if [ $status -ne 0 ]; then
  cat "$work/test.out"
  exit 1
fi

echo "== builds"
mkdir -p "$work/dev" "$work/release"
(cd "$work/dev" && "$KANSO" build "$here/main.kso" > /dev/null) || exit 1
(cd "$work/release" && "$KANSO" build "$here/main.kso" --release > /dev/null) \
  || exit 1

echo "== fixtures"
total=0
failed=0
for md in "$here"/fixtures/*/*.md; do
  stem=${md%.md}
  want="$stem.html"
  [ -f "$want" ] || continue
  total=$((total + 1))
  args=""
  [ -f "$stem.args" ] && args=$(cat "$stem.args")
  # shellcheck disable=SC2086
  "$KANSO" run "$here/main.kso" --interp -- $args < "$md" > "$work/interp" 2>&1
  # shellcheck disable=SC2086
  "$work/dev/main" $args < "$md" > "$work/dev.out" 2>&1
  # shellcheck disable=SC2086
  "$work/release/main" $args < "$md" > "$work/release.out" 2>&1
  bad=""
  cmp -s "$work/interp" "$want" || bad="$bad interp"
  cmp -s "$work/dev.out" "$want" || bad="$bad dev"
  cmp -s "$work/release.out" "$want" || bad="$bad release"
  if [ -n "$bad" ]; then
    failed=$((failed + 1))
    echo "FAIL ${md#"$here"/}:$bad"
  fi
done

echo "$((total - failed)) of $total fixtures agree on all three engines"
[ $failed -eq 0 ]
