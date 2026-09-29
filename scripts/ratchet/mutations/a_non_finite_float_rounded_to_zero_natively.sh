#!/bin/sh
# Round NaN and the infinities to 0 in the compiled runtime, where every engine
# answers `none` since 2026-09-29. Native then prints 0 where the interpreter
# prints <none>, and the micro corpus goes red.
set -e
old='        if (x != x || isinf(x)) return k_none();'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        if (x != x || isinf(x)) return k_none();$#        if (x != x || isinf(x)) return k_int(0);#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
