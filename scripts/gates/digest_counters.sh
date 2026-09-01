#!/bin/sh
# The only benchmark whose PEAK is the point.
#
# arena_peak_bytes is the row to read. sha256 walks a padded message
# sixty-four bytes at a time carrying eight state words, and everything a
# block builds is dead when the next starts — so this number is a constant in
# a compiler that reclaims between blocks and linear in the message in one
# that does not. It reads 82,837,504 for an 8,192-byte message today, which is
# about ten thousand bytes an input byte, and that is the second case.
#
# A FALL here is the win the benchmark exists to collect and wants saying so
# in design/compiler-log.md. A rise is the walk holding more per block.
#
# It exists because on 2026-08-31 a change took this number to 1,048,576 and
# scored zero against welfare, which weighed nothing in this shape; the same
# change was 52x slower in wall clock and nothing in the corpus was long
# enough on an input like this to notice either.
set -e
KANSO_COUNTERS=1 ./digestbench 2>counters_digest.txt >/dev/null
diff bench/cost_golden_digest.txt counters_digest.txt || {
  echo "::error::digest counters diverged. arena_peak_bytes is the row"
  echo "::error::to read: it is what a streaming walk holds, and it is"
  echo "::error::linear in the message rather than constant. A FALL is"
  echo "::error::the win this benchmark exists to collect — regenerate"
  echo "::error::the golden and say so in design/compiler-log.md."
  exit 1
}
