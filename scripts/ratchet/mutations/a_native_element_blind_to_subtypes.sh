#!/bin/sh
# Stop a compiled `join` and `to_bytes` reading an element through a
# subtype, as they did not until 2026-09-28. Each then refuses a list of
# subtypes, and the micro fixture a_list_of_subtypes_joins dies where the
# interpreter prints.
set -e
f=src/runtime.c
old1='        if (l->items[i].tag != K_STR) return k_join_wrapped(l, ss, i);'
old2='            if (item.tag == K_SUB && b.tag == K_INT && b.payload >= 0 && b.payload <= 255) {'
[ "$(grep -cxF "$old1" $f)" -eq 1 ]
[ "$(grep -cxF "$old2" $f)" -eq 1 ]
sed -i.bak -e 's#^        if (l->items\[i\].tag != K_STR) return k_join_wrapped(l, ss, i);$#        if (l->items[i].tag != K_STR) k_die("join takes a list of strings");#' \
  -e 's#^            if (item.tag == K_SUB \&\& b.tag == K_INT \&\& b.payload >= 0 \&\& b.payload <= 255) {$#            if (0) {#' $f
rm -f $f.bak
! grep -qxF "$old1" $f
! grep -qxF "$old2" $f
