#!/bin/sh
# The library's escaper drops its fast path: a string the scan proved clean is
# handed to the found path anyway, so its first byte goes through `esc_byte`
# on its own, the scan runs a second time from the second byte, and the rest
# leaves as a slice. Two scans and two copies where one of each would do.
#
# This is the defect livebench was added for. bench/encodebench runs the same
# program four hundred times over and would not move by one instruction,
# because it vendors a snapshot of lib/json frozen at 20ab931d on purpose.
# bench/oneshot imports the real library and encodes exactly once, so it sees a
# sliver. The live vein sees all of it: `find2_calls` rises by one per clean
# string and `append_fast` moves with the split copy.
#
# The arm is repointed rather than deleted, so the program still answers the
# same bytes and the checksum gate stays green -- a mutation that changed the
# OUTPUT would turn this gate red for a reason that has nothing to do with the
# counters it exists to watch. `s` is re-encoded rather than dropped for a
# duller reason: the language refuses a body that leaves a parameter unused, so
# a mutation that simply deleted the fast path did not compile, and a build that
# never runs is UNBUILT rather than red. Until 2026-09-07 this patched the
# clean arm to the fold; the fold left the library that day and the row went
# UNBUILT for a round.
set -e
grep -q '^  if (len < n) (text/append acc s) (escape_rest acc bs n len)$' \
  lib/json/text.kso || {
  echo "lib/json's escape fast path changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  if (len < n) (text/append acc s) (escape_rest acc bs n len)$|  if (len < n) (escape_rest acc (text/bytes s) 1 len) (escape_rest acc bs n len)|' \
  lib/json/text.kso
grep -q 'escape_rest acc (text/bytes s) 1 len' lib/json/text.kso
