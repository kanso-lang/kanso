#!/bin/sh
# The benchmarks bench/benchmarks.txt lists, one name to a line. Not a gate.
#
#   sh scripts/gates/benchmarks.sh           every benchmark
#   sh scripts/gates/benchmarks.sh counted   the ones with a counter gate
set -e
list="$(dirname "$0")/../../bench/benchmarks.txt"
case "$1" in
  counted) sed -e '/^#/d' -e '/ plain$/d' "$list" ;;
  *) sed -e '/^#/d' -e 's/ .*//' "$list" ;;
esac
