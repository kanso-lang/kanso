#!/bin/sh
# The traffic the front end makes, which no other gate can see. Rounds and
# visits count the work the compiler decided to do; peak counts what it held.
# A pass that allocates a String per identifier occurrence and one that borrows
# the program's names agree on all three, because the strings are transient —
# so a quarter off a pass's time landed with every gate reporting nothing.
set -e
golden=bench/compile_allocs_golden.txt
sh scripts/gates/measured_on.sh "$golden"
KANSO_COUNTERS=1 ./target/release/kanso check lib/json 2>counters_allocs.txt >/dev/null
grep -v '^#' "$golden" > allocs_want.txt
for k in compile_allocs; do
  grep "^${k}=" counters_allocs.txt
done > allocs_got.txt
diff allocs_want.txt allocs_got.txt || {
  echo "::error::the front end's allocation traffic moved. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank — say"
  echo "::error::which in design/compiler-log.md and regenerate"
  echo "::error::$golden. Rounds, visits and peak cannot see this"
  echo "::error::dimension, which is why it has a vein of its own."
  exit 1
}
