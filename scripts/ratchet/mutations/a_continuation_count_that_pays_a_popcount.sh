#!/bin/sh
# The wide pass counts each block's continuation bytes the way it did before:
# a movemask and a popcount, which the ssse3 build writes out as bit
# arithmetic. The count is the same and the block costs twenty more
# instructions.
set -e
f=src/runtime.c
grep -q 'counted = _mm_add_epi64(counted, _mm_sad_epu8(ones, _mm_setzero_si128()));' src/runtime.c
sed -i 's/counted = _mm_add_epi64(counted, _mm_sad_epu8(ones, _mm_setzero_si128()));/counted = _mm_add_epi64(counted, _mm_cvtsi64_si128(__builtin_popcount((unsigned)_mm_movemask_epi8(cont) + 0 * _mm_cvtsi128_si32(ones))));/' "$f"
if grep -q '_mm_sad_epu8(ones, _mm_setzero_si128())' "$f"; then exit 1; fi
