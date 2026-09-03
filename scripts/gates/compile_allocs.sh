#!/bin/sh
# The traffic the front end makes, which no other gate can see. Rounds and
# visits count the work the compiler decided to do; peak counts what it held.
# A pass that allocates a String per identifier occurrence and one that borrows
# the program's names agree on all three, because the strings are transient —
# so a quarter off a pass's time landed with every gate reporting nothing.
set -e
golden=bench/compile_allocs_golden.txt
# A host the golden does not name still MEASURES on CI, so the job log carries
# the sitting the refusal tells a reader to copy. scripts/gates/host_gate.sh
# carries the reasons; 3 means measure, print, and fail at the end.
host=0
sh scripts/gates/host_gate.sh "$golden" || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi
sh scripts/gates/library_box.sh
(cd /tmp/kanso-compile-ir && KANSO_COUNTERS=1 ./kanso check lib/json 2>&1 >/dev/null) \
  > counters_allocs.txt
grep -v '^#' "$golden" > allocs_want.txt
for k in compile_allocs; do
  grep "^${k}=" counters_allocs.txt
done > allocs_got.txt

# Measured, and on a host the golden does not name that is as far as this goes:
# the rows are printed for CI's job log and nothing is compared against them.
if [ "$host" -eq 3 ]; then
  echo "::error::this runner's sitting, to copy into $golden together with its"
  echo "::error::measured-on line. NOTHING IS COMPARED:"
  sed 's/^/::error::    /' allocs_got.txt
  exit 1
fi

diff allocs_want.txt allocs_got.txt || {
  echo "::error::the front end's allocation traffic moved. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank — say"
  echo "::error::which in design/compiler-log.md and regenerate"
  echo "::error::$golden. Rounds, visits and peak cannot see this"
  echo "::error::dimension, which is why it has a vein of its own."
  exit 1
}
