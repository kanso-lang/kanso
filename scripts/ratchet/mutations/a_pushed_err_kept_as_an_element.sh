#!/bin/sh
# Stop the C push asking whether its item is an err, as before 2026-09-28.
#
# A push the analysis does not own stores the err as an element where the
# interpreter hands it on, and the micro fixture prints a list holding one.
set -e
sed -i.bak 's/if (__builtin_expect(!k_not_failure(lv) || !k_not_failure(item), 0)) return k_failed2(lv, item);/if (!k_not_failure(lv)) return lv;/' src/runtime.c
rm -f src/runtime.c.bak
test "$(grep -c 'k_failed2(lv, item)' src/runtime.c)" = 0
