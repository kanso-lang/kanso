#!/bin/sh
# Call a three-argument closure from the runtime with its arguments as KValue
# structs again, as k_call3 did until 2026-09-29. C then hands the third one
# to the stack whole while the lambda reads half of it from a register, a
# partial over a lambda of three answers garbage, and the micro corpus goes
# red.
set -e
old1='        return ((KValue(K_CLOSCC *)(void*, W, W, W, W, W, W))cl->fn)('
old2='            cl->env, a.tag, a.payload, b.tag, b.payload, c.tag, c.payload);'
[ "$(grep -cxF "$old1" src/runtime.c)" -eq 1 ]
[ "$(grep -cxF "$old2" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        return ((KValue(K_CLOSCC \*)(void\*, W, W, W, W, W, W))cl->fn)($#        return ((KValue(K_CLOSCC *)(void*, KValue, KValue, KValue))cl->fn)(#' src/runtime.c
sed -i.bak 's#^            cl->env, a.tag, a.payload, b.tag, b.payload, c.tag, c.payload);$#            cl->env, a, b, c);#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old1" src/runtime.c
! grep -qxF "$old2" src/runtime.c
