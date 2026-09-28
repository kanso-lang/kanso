#!/bin/sh
# Stop the C push asking whether its item is an err, as before 2026-09-28.
#
# A push onto a list the analysis does not own stores the err as an element
# where the interpreter hands it on, and the micro fixture prints the list.
set -e
sed -i.bak '/^    if (!k_not_failure(item)) return item;$/d' src/runtime.c
rm -f src/runtime.c.bak
! grep -q '^    if (!k_not_failure(item)) return item;$' src/runtime.c
