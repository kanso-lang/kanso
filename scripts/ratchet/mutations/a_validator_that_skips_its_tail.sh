#!/bin/sh
# The utf-8 validator never looks at the last bytes of a long run.
#
# k_all_ascii reads whole words while eight or more remain, then reads the
# LAST eight — overlapping bytes the loop already saw rather than walking
# whatever is left one at a time. Pointing that final read back at the start
# re-reads the first eight instead, so for a run of nine to fifteen bytes
# nothing ever looks past byte eight and a continuation byte hiding there is
# called valid.
#
# This mutation exists because the harness could not see it. Its sampled band
# ran from four to eight bytes, which is exactly the region where the word
# loop does all the work and the tail has nothing to answer for; the mutation
# passed 45,189,025 cases with zero mismatches. The band now runs to
# twenty-four and it fails on the first hundred, e.g.
# `MISMATCH len=12 bytes=25 32 71 15 15 57 29 72 15 e4 5d 13 got=1 want=0`.
#
# In the scalar predicate rather than either vector body, so the mutation
# reads the same on x86 and on arm.
set -e
f=src/runtime.c
# THE PATH IS SPELLED ON A GUARD LINE, and that is load-bearing rather than
# decorative. The ratchet's `touched` pass selects the rows a branch could have
# made blind by intersecting the files a branch changed with the paths each
# mutation names on a guard line. Every assertion below reaches the file
# through "$f", so the path appeared on no guard line and this row was
# invisible to that pass whatever a branch touched -- the same defect
# kanso#1338 repaired on the two compile-path mutations.
grep -q 'k_utf8_check' src/runtime.c
grep -qF 'memcpy(&w, data + len - 8, sizeof w);' "$f" || {
  echo "the overlapping tail read moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|memcpy(&w, data + len - 8, sizeof w);|memcpy(\&w, data + 0, sizeof w);|' "$f"
grep -qF 'memcpy(&w, data + 0, sizeof w);' "$f"
