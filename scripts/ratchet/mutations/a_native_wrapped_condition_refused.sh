#!/bin/sh
# Stop a native condition reading through a subtype of bool, as no engine did
# until 2026-09-28. The inline test then refuses the wrapper, and the micro
# fixture a_subtype_of_bool_is_a_condition dies where the interpreter prints.
set -e
old='    if (v.tag == K_SUB) {'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    if (v.tag == K_SUB) {$#    if (0) {#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
