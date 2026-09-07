#!/bin/sh
# The run in front of a string's first escape is walked byte by byte through
# the fold again, as it was before 2026-09-07 -- bytes the scan had already
# proved clean. The output is the same bytes, so the checksum gate stays green;
# what moves is the live vein's counters: `append_fast` rises by the bytes the
# fold walks again and the suffix slice's allocation goes away, which is what
# this row proves the gate can see. Repointed at the `escape_split` arm rather
# than gutting `escape_rest`, because an arm left with an unused parameter does
# not compile, and a build that never runs is UNBUILT rather than red.
set -e
grep -q '^  if (len < n) (text/append acc s) (escape_rest acc bs n len)$' \
  lib/json/text.kso || {
  echo "lib/json's escape split changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  if (len < n) (text/append acc s) (escape_rest acc bs n len)$|  if (len < n) (text/append acc s) (escape_able acc bs)|' \
  lib/json/text.kso
grep -q '(text/append acc s) (escape_able acc bs)' lib/json/text.kso
