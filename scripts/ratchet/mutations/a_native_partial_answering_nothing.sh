#!/bin/sh
# Answer no count for a partial in the native runtime, as it did until
# 2026-09-29. `s = &k` over `k = &h 3` then grows at `s 1` and prints `<fn>`
# where the interpreter runs g, and the micro corpus goes red.
set -e
old='        if (c->arity >= 0 || c->fn) return c->arity;'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        if (c->arity >= 0 || c->fn) return c->arity;$#        return c->arity;#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
