#!/bin/sh
# Give native's stack of open ties the fixed 256 entries it had before
# 2026-10-05. three_hundred_ties_nested then stops at the 257th where the
# interpreter prints the walk, and the micro corpus goes red.
set -e
old='    if (k_tie_depth == k_tie_cap) {'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    if (k_tie_depth == k_tie_cap) {$#    if (k_tie_depth >= 256) k_die("list/tie nests deeper than the runtime holds"); if (k_tie_depth == k_tie_cap) {#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$old" src/runtime.c
