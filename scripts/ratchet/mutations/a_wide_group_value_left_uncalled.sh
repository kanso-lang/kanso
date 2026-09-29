#!/bin/sh
# Stop the native dispatcher from calling a group value with more than four
# arguments, as it could not until 2026-09-29. A partial over a group of five
# or more then dies where it should run the group, and the micro corpus goes
# red.
set -e
old='    if (callee.tag == K_FNREF) return k_call_ref_wide((KFnref*)(intptr_t)callee.payload, n, args);'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    if (callee.tag == K_FNREF) return k_call_ref_wide((KFnref\*)(intptr_t)callee.payload, n, args);$#    if (0) return k_none();#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
