#!/bin/sh
# Let a map be indexed by any key, as a compiled build did until 2026-09-28.
# A none compares against the map's keys and answers a miss, and the runtime
# corpus fixture a_map_indexed_by_a_none_is_refused prints where the
# interpreter refuses.
set -e
old='        if (index.tag != K_INT && index.tag != K_STR) {'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        if (index.tag != K_INT \&\& index.tag != K_STR) {$#        if (0) {#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
