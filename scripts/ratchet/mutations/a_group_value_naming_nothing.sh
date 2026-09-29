#!/bin/sh
# Answer a group value's failing argument without naming the group, as the
# native dispatchers did until 2026-09-29. `f = f1; f 0 bad` then drops f1
# from the trace the interpreter prints, and the runtime corpus goes red.
set -e
old='    return r->builtin ? failure : k_err_hop(failure, r->name);'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    return r->builtin ? failure : k_err_hop(failure, r->name);$#    return failure;#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
