#!/bin/sh
# The only benchmark whose cost grows with the SUBJECT rather than with the
# program text. escapebench watches storage the arena never held; this watches
# storage the arena holds and never rewinds, which looks identical from outside
# the process and has nothing in common inside it.
#
# arena_blocks is the row to read. A scan that rewinds per position keeps a
# handful of blocks whatever the subject; this one keeps 4, 12, 48 and 189 for
# subjects of 125, 250, 500 and 1,000 characters, quadrupling as the subject
# doubles. Nothing else in the corpus can see that shape, so a change that
# restores per-position rewinding shows up here and only here.
set -e
KANSO_COUNTERS=1 ./scanbench 2>counters_scan.txt >/dev/null
diff bench/cost_golden_scan.txt counters_scan.txt || {
  echo "::error::scan counters diverged. arena_blocks is the row to read"
  echo "::error::first: it is arena the scan never gave back, and it grows"
  echo "::error::with the square of the subject. A FALL is the win this"
  echo "::error::benchmark exists to collect — regenerate the golden and"
  echo "::error::say so. A rise is the scan retaining more per position."
  exit 1
}
