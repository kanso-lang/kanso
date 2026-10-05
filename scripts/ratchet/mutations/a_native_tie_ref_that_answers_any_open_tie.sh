#!/bin/sh
# Let native's `ref` answer whichever tie is open, as naming its tie by
# position did before 2026-10-05. A `ref` called after its tie returned, from
# inside a second tie's maker, then answers the second tie's cell instead of
# being refused, and the runtime corpus goes red on native.
set -e
old='        if (k_ties[i].serial == serial) t = &k_ties[i];'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        if (k_ties\[i\].serial == serial) t = &k_ties\[i\];$#        if (k_ties[i].serial == serial || serial > 0) t = \&k_ties[i];#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
