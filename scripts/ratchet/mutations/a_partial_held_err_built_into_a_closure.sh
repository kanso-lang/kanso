#!/bin/sh
# Ask none of a partial's held arguments whether it failed, as native did
# until 2026-09-29. `&g9 p v 1 2 3` with `v` an err is then a function rather
# than the err, and the micro corpus goes red. Since native holds every
# partial over a declared group over the group's value, the question is asked
# in k_partial1 to k_partial4, and this takes it out of all four.
set -e
first='KValue k_partial1(KValue f, KValue a) {'
[ "$(grep -cxF "$first" src/runtime.c)" -eq 1 ]
sed -i.bak '/^KValue k_partial1(KValue f, KValue a) {$/,/^    return k_partial_build(f, 4, args);$/ { /^    if (!k_not_failure([abcd])) return [abcd];$/d; }' src/runtime.c
rm -f src/runtime.c.bak
! sed -n '/^KValue k_partial1(KValue f, KValue a) {$/,/^    return k_partial_build(f, 4, args);$/p' src/runtime.c | grep -qE '^    if \(!k_not_failure\([abcd]\)\) return [abcd];$'
