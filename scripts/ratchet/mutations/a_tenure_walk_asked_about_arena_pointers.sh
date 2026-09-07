#!/bin/sh
# k_survives_x asks whether a pointer the rewind would reclaim was tenured by
# an earlier lap. Before 2026-09-07 it asked the tenure blocks straight away,
# and the sizing walk asks it for every node the loop built this lap -- a
# pointer that sits in the arena above the mark and cannot be tenured, since
# tenure blocks are malloc'd. Under runbench an inner loop handed forty-nine
# 1,600-byte tenure blocks up to the loop outside it, and every one of 375,922
# asks walked all forty-nine: 224 instructions an ask, 2.99% of the program.
# The short-circuit answers those asks from the arena chain, one or two blocks
# from the head. This mutation puts the walk back in front of it.
#
# No counter can see this. The blocks are the same, the hand-ups are the same,
# ten_blocks and ten_frees read 55 either way, and the emitted code is the
# compiler's, not the runtime's. The work vein is the witness: runbench
# 2,816,922,233 -> 2,762,899,581 on the container when the short-circuit
# landed, and it goes back up when this runs.
set -e
grep -q '^    if (k_above_mark(p, m)) return 0;$' src/runtime.c || {
  echo "k_survives_x's above-mark short-circuit changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    if (k_above_mark(p, m)) return 0;$|    if (0 \&\& k_above_mark(p, m)) return 0;|' \
  src/runtime.c
grep -q '^    if (0 && k_above_mark(p, m)) return 0;$' src/runtime.c
