#!/bin/sh
# k_b_to_int parses [-]?digits in a bare loop when the digit run is eighteen
# or fewer, because eighteen digits cannot overflow an i64. Nineteen can:
# 9,999,999,999,999,999,999 against i64's 9,223,372,036,854,775,807. Widening
# the bound is the edit someone optimising this would reach for, and the loop
# then wraps silently where libc saturates and the caller raises.
#
# Nothing in the tree could see this before 2026-09-14. The corpus pins the
# integers it happens to parse and none of them is nineteen digits.
set -e
t='    if (start < len && len - start <= 18) {'
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "to_int's fast-path bound changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's#^    if (start < len \&\& len - start <= 18) {$#    if (start < len \&\& len - start <= 19) {#' src/runtime.c
grep -qF '    if (start < len && len - start <= 19) {' src/runtime.c
