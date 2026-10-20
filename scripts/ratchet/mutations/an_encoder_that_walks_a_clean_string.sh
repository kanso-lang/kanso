#!/bin/sh
# The library's escaper drops its fast path: a string the scan proved clean is
# handed to `append_escaped` anyway, which writes the same bytes through its
# own copy instead of `append`'s in-place one.
#
# This is the defect livebench was added for. bench/encodebench runs the same
# program four hundred times over and would not move by one instruction,
# because it vendors a snapshot of lib/json frozen at 20ab931d on purpose.
# bench/oneshot imports the real library and encodes exactly once, so it sees a
# sliver. The live vein sees all of it: `append_fast` falls by one per clean
# string, since the escaping door writes into the builder without it.
#
# The arm is repointed rather than deleted, so the program still answers the
# same bytes and the checksum gate stays green -- a mutation that changed the
# OUTPUT would turn this gate red for a reason that has nothing to do with the
# counters it exists to watch. Until 2026-10-20 this repointed the clean arm
# to the kanso escaper, which left the library that day for `append_escaped`.
set -e
grep -q '^  if (len < n) (text/append acc s) (builtin_append_escaped acc s)$' \
  lib/json/text.kso || {
  echo "lib/json's escape fast path changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  if (len < n) (text/append acc s) (builtin_append_escaped acc s)$|  if (len < n) (builtin_append_escaped acc s) (builtin_append_escaped acc s)|' \
  lib/json/text.kso
grep -q '^  if (len < n) (builtin_append_escaped acc s) (builtin_append_escaped acc s)$' lib/json/text.kso
