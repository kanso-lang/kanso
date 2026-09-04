#!/bin/sh
# The only benchmark in the corpus that reads its input from a file with the
# bang, and the only one whose arena grows with the number of rounds when the
# read's yield is unknown.
#
# WHY IT EXISTS. bench/make_jsonbench used to write `os/read_file` plus a
# `file_not_found` arm, and its comment said why: the bang cost the loop its
# beat, because inference read a chain's yield off the head's bare name and
# `os/read_file!` fell to the top set. The document then had no type where the
# loop analysis reads one and the decode ran on a grow-only arena. The corpus
# was measuring the workaround, so when the yield was fixed every counter in
# every golden was byte-identical and the objective scored the repair at zero.
#
# This benchmark writes the spelling the workaround avoided. On 159f6b2b it
# reads arena_blocks=41 and arena_peak_bytes=42,991,616; with the yield carried
# per declaration it reads 1 and 1,048,576, on the same bytes and the same
# loop. beat_iters is the row that says which: 1 means the loop never bracketed
# at all, 201 means it bracketed every round.
set -e
KANSO_COUNTERS=1 ./readbench 2>counters_read.txt >/dev/null
diff bench/cost_golden_read.txt counters_read.txt || {
  echo "::error::read counters diverged. beat_iters is the row to read"
  echo "::error::first: 201 is the loop bracketing every round and 1 is it"
  echo "::error::never bracketing, which puts arena_blocks at 41 and the"
  echo "::error::peak at 41 MB. A fall to 1 there is the beat going away."
  exit 1
}
