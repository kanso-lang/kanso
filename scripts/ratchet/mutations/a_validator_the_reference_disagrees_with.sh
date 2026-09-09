#!/bin/sh
# The utf-8 validator accepts a byte the independent reference refuses.
#
# k_utf8_bad opens by asking k_all_ascii, which answers the whole question for
# a run with no high bit in it and never reaches the wide classification. Its
# narrowest arm is the single byte, and the bound there is what makes it an
# ascii test. Moving it one value up admits 0x80, which is a continuation byte
# with nothing in front of it and not a character at all.
#
# The sweep is exhaustive over every string of three bytes or fewer, so it
# reaches this on the first length: `MISMATCH len=1 bytes=80 got=1 want=0`.
#
# Deliberately in the scalar predicate rather than in either vector body, so
# the mutation reads the same on x86 and on arm — the ratchet must not depend
# on which host it runs.
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
grep -qF 'if (len == 1) return (uint8_t)data[0] < 0x80;' "$f" || {
  echo "the ascii predicate moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|if (len == 1) return (uint8_t)data\[0\] < 0x80;|if (len == 1) return (uint8_t)data[0] <= 0x80;|' "$f"
grep -qF 'if (len == 1) return (uint8_t)data[0] <= 0x80;' "$f"
