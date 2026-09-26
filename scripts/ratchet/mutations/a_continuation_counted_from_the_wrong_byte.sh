#!/bin/sh
# The wide pass's continuation compare moves its bound by one, so 0xBF is no
# longer counted as a continuation byte and every string holding one is given
# a character count too high by one for each.
set -e
f=src/runtime.c
grep -q '__m128i cont = _mm_cmpgt_epi8(_mm_set1_epi8((char)-64), cur);' src/runtime.c
sed -i 's/__m128i cont = _mm_cmpgt_epi8(_mm_set1_epi8((char)-64), cur);/__m128i cont = _mm_cmpgt_epi8(_mm_set1_epi8((char)-65), cur);/' "$f"
if grep -q '_mm_set1_epi8((char)-64), cur);' "$f"; then exit 1; fi
