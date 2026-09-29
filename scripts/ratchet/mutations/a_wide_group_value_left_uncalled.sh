#!/bin/sh
# Stop the native dispatcher at four arguments again, as it did until
# 2026-09-29. A partial over a group of five or more then dies where it should
# run the group, and the micro corpus goes red.
set -e
old='    if (n <= 8) {'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    if (n <= 8) {$#    if (0) {#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
