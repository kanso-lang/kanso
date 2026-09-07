#!/bin/sh
# The bind chain sizes its continuation on every step, and the walk's
# closure, description and subtype arms recursed on every slot, an int or
# a string as often as a pointer: 176,113 of the 510,001 slots the run
# program's walk visited were immediates, each paying a call whose first
# test sent it back. Since 2026-09-07 those arms ask k_worth_sizing first,
# as the list arm has since deepbench, and the seen-map probe is inlined
# at its sites. This mutation puts the unconditional calls and the
# out-of-line probe back. The bytes out are the same and no counter moves,
# so the work vein is the witness: runbench 2,453,160,233 -> 2,446,395,268
# on the container.
set -e
n=$(grep -cF 'if (k_worth_sizing(((KValue*)cl->env)[i]))' src/runtime.c)
[ "$n" -eq 1 ] || { echo "the closure arm changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^                if (k_worth_sizing(((KValue\*)cl->env)\[i\]))$|                if (1)|' src/runtime.c
sed -i 's|^            if (k_worth_sizing(d->x)) n += k_copy_size(d->x, m);$|            n += k_copy_size(d->x, m);|' src/runtime.c
sed -i 's|^            if (k_worth_sizing(d->y)) n += k_copy_size(d->y, m);$|            n += k_copy_size(d->y, m);|' src/runtime.c
sed -i 's|^            if (k_worth_sizing(sb->inner)) n += k_copy_size(sb->inner, m);$|            n += k_copy_size(sb->inner, m);|' src/runtime.c
python3 - <<'PY'
p='src/runtime.c'; s=open(p).read()
for fn in ('size_t k_ptrmap_probe(KPtrMap* t, const void* key) {',
           'KPtrSlot* k_ptrmap_at(KPtrMap* t, const void* key, size_t* live) {'):
    a='static inline __attribute__((always_inline))\n'+fn
    assert s.count(a)==1, fn
    s=s.replace(a,'static '+fn)
open(p,'w').write(s)
PY
grep -qF '                if (1)' src/runtime.c
grep -qF 'static size_t k_ptrmap_probe(KPtrMap* t, const void* key) {' src/runtime.c
! grep -qF 'if (k_worth_sizing(d->x))' src/runtime.c
