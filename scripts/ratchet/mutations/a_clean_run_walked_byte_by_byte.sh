#!/bin/sh
# A clean run between two escapes is walked byte by byte again, one append per
# byte, as the fold did before 2026-09-07 -- bytes the scan had already proved
# clean. The output is the same bytes, so the checksum gate stays green; what
# moves is the live vein's counters: `append_fast` rises by the bytes walked
# again and `find2_calls` holds, which is what this row proves the gate can
# see. The mutation is one line and imports nothing, because a walk written as
# a self-call needs no fold and the arm keeps every parameter in use; an arm
# left with an unused parameter does not compile, and a build that never runs
# is UNBUILT rather than red.
set -e
grep -q '^  if (p < n) (text/append acc (text/slice bs p (n - 1))) acc$' \
  lib/json/text.kso || {
  echo "lib/json's escape run changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  if (p < n) (text/append acc (text/slice bs p (n - 1))) acc$|  if (p < n) (escape_run (text/append acc bs[p]!) bs (p + 1) n) acc|' \
  lib/json/text.kso
grep -q '(escape_run (text/append acc bs\[p\]!) bs (p + 1) n) acc' lib/json/text.kso
