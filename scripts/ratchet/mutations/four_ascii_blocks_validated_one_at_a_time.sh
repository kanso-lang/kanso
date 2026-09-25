#!/bin/sh
# The wide utf-8 pass validates sixteen bytes a block, and an all-ascii
# block skips the classification: twelve instructions to learn that sixteen
# bytes of an encoder's output need nothing. Since 2026-09-07 four ascii
# blocks are passed in one test with one mask. This mutation walks them one
# at a time again. The bytes out are the same and no counter moves, so the
# work vein is the witness.
set -e
target='        while (i + 64 <= len && !_mm_movemask_epi8(prev)) {'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_utf8_bad_wide changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        while (i + 64 <= len \&\& !_mm_movemask_epi8(prev)) {$|        while (0) {|' src/runtime.c
! grep -qF "$target" src/runtime.c
