#!/bin/sh
# A run of thirty-two bytes or fewer that carries a byte with the high bit set
# is validated by the scalar arm, and until 2026-09-07 that arm walked the
# ascii around the multi-byte characters one byte at a time: 190 instructions
# a run on runbench's 134,442 short strings, most of them ascii. The arm reads
# a word at a time now, as the door's predicate does, lands on the first byte
# with a high bit by counting trailing zeros, and reads the tail as the run's
# last word with the walked bytes shifted out. This mutation closes both word
# arms, so every ascii byte is a step again. The answer is the same either
# way and no counter moves -- utf8_bytes counts bytes handed in, not steps --
# so the work vein is the witness: runbench 2,674,319,744 -> 2,665,704,368 on
# the container when the words landed. The differential harness under
# scripts/utf8_differential checks the arm's text against an independent
# reference, and it passed 45,189,025 cases with 0 mismatches either way.
set -e
n=$(grep -c '^            if (i + 8 <= len) {$' src/runtime.c)
m=$(grep -c '^            if (len >= 8) {$' src/runtime.c)
[ "$n" -eq 1 ] && [ "$m" -eq 1 ] || {
  echo "the scalar arm's word reads changed shape ($n, $m); rewrite this" >&2
  exit 1
}
sed -i 's|^            if (i + 8 <= len) {$|            if (0 \&\& i + 8 <= len) {|' src/runtime.c
sed -i 's|^            if (len >= 8) {$|            if (0 \&\& len >= 8) {|' src/runtime.c
grep -q '^            if (0 && i + 8 <= len) {$' src/runtime.c
grep -q '^            if (0 && len >= 8) {$' src/runtime.c
