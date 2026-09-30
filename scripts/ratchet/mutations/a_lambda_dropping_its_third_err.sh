#!/bin/sh
# Merge only the first two failing slots of a three-argument lambda, as native
# did before 2026-09-30. Errs in the first and third slots then answer the
# first alone where the interpreter lists both, and the runtime corpus goes
# red.
set -e
old='        if (k_any_failure(3, xs)) return k_failed_of(3, xs);'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        if (k_any_failure(3, xs)) return k_failed_of(3, xs);$#        if (k_any_failure(3, xs)) return k_failed_of(2, xs);#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
