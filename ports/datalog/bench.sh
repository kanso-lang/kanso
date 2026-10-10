#!/bin/sh
# Times the largest fixtures on all three engines. Each figure is the
# program's own `--time` report: wall-clock milliseconds from reading the
# source to printing the last line, on standard error. Compilation is not
# included.
#
#   sh bench.sh                     tc_large and same_generation_large
#   sh bench.sh tests/foo.dl ...    other programs
set -u
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here" || exit 1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
"$KANSO" build main.kso > /dev/null || exit 1
mv main "$tmp/datalog-dev"
"$KANSO" build main.kso --release > /dev/null || exit 1
mv main "$tmp/datalog-release"
rm -f main.ll
[ $# -eq 0 ] && set -- tests/tc_large.dl tests/same_generation_large.dl
for program in "$@"; do
  echo "== $program"
  grep "^total:" "${program%.dl}.expected" 2>/dev/null || true
  for engine in interp dev release; do
    case $engine in
      interp) set -- "$KANSO" run main.kso --interp -- ;;
      dev) set -- "$tmp/datalog-dev" ;;
      release) set -- "$tmp/datalog-release" ;;
    esac
    took=$("$@" --time "$program" 2>&1 > /dev/null | grep '^time:')
    printf '%-8s %s\n' "$engine" "$took"
  done
done
