#!/bin/sh
# k_where places a node against a beat's mark in one walk of the block
# chain. Until 2026-09-07 it walked from the head, so a node in the mark's
# own block -- above the mark if the loop built it this lap, below if the
# lap before left it, and those two are most of what the sizing walk asks
# about -- was reached only after every block newer than the mark's. Since
# then the mark's block is asked first, then the newer blocks, then the
# older. This mutation puts the walk from the head back. The bytes out are
# the same and no counter moves, so the work vein is the witness: pendbench
# 608,937,982 -> 604,694,569 and runbench 2,487,359,798 -> 2,483,621,153 on
# the container when the mark's block came first.
set -e
n=$(grep -cF '    KBlock* mb = m->block;' src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_where changed shape ($n); rewrite this" >&2; exit 1; }
sed -i '/^    KBlock\* mb = m->block;$/,/^    return K_WHERE_OUTSIDE;$/c\
    int below = 0;\
    for (KBlock* b = k_blocks; b; b = b->next) {\
        const char* start = (const char*)(b + 1);\
        const char* end = start + b->cap;\
        if (b == m->block) {\
            if (q >= start && q < m->ptr) return K_WHERE_BELOW;\
            if (q >= m->ptr && q < end) return K_WHERE_ABOVE;\
            below = 1;\
            continue;\
        }\
        if (q >= start && q < end) return below ? K_WHERE_BELOW : K_WHERE_ABOVE;\
    }\
    return K_WHERE_OUTSIDE;' src/runtime.c
grep -qF 'if (q >= start && q < end) return below ? K_WHERE_BELOW : K_WHERE_ABOVE;' src/runtime.c
! grep -qF '    KBlock* mb = m->block;' src/runtime.c
