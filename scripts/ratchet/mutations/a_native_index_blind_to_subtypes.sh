#!/bin/sh
# Stop a compiled index reading through a subtype, as it did not for a list
# or a string until 2026-09-28. A subtype of int is then refused as a
# position, and the micro fixture a_subtype_indexes_as_its_base dies where
# the interpreter prints.
set -e
old='    if (container.tag == K_SUB || index.tag == K_SUB) {'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    if (container.tag == K_SUB || index.tag == K_SUB) {$#    if (0) {#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
