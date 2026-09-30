#!/bin/sh
# Answer a group value's failing arguments before entering the group, without
# naming it, as the native dispatchers did before 2026-09-29. `f = f1; f 0 bad`
# then drops f1 from the trace the interpreter prints, `f bad worse` merges
# two errs a direct call reports one of, and the runtime corpus goes red.
set -e
old='        if (r->builtin && (!k_not_failure(a) || !k_not_failure(b))) return k_both_or_either(a, b);'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        if (r->builtin \&\& (!k_not_failure(a) || !k_not_failure(b))) return k_both_or_either(a, b);$#        if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
