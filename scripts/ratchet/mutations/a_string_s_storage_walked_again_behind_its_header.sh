#!/bin/sh
# A string built by k_str_alloc keeps its bytes right after its header, so
# when the sizing walk has just found the header not to survive, the bytes
# do not survive either, and the walk asked of them is the walk it just
# made. Since 2026-09-07 the K_STR and K_BYTES arms ask it only of storage
# that lives elsewhere -- a slice's, a builder's. This mutation asks it of
# every string again. The bytes out are the same either way and no counter
# moves, so the work vein is the witness: runbench 2,606,982,659 ->
# 2,602,519,000 on the container when the shortcut landed.
set -e
a='            if (s->data == (char*)(s + 1) || !k_survives_x(s->data, m))'
b='            if (b->data == (const unsigned char*)(b + 1) || !k_survives_x(b->data, m))'
[ "$(grep -cF "$a" src/runtime.c)" -eq 1 ] || { echo "the string arm changed shape; rewrite this" >&2; exit 1; }
[ "$(grep -cF "$b" src/runtime.c)" -eq 1 ] || { echo "the bytes arm changed shape; rewrite this" >&2; exit 1; }
sed -i 's|^            if (s->data == (char\*)(s + 1) \|\| !k_survives_x(s->data, m))$|            if (!k_survives_x(s->data, m))|' src/runtime.c
sed -i 's|^            if (b->data == (const unsigned char\*)(b + 1) \|\| !k_survives_x(b->data, m))$|            if (!k_survives_x(b->data, m))|' src/runtime.c
grep -qF '            if (!k_survives_x(s->data, m))' src/runtime.c
grep -qF '            if (!k_survives_x(b->data, m))' src/runtime.c
