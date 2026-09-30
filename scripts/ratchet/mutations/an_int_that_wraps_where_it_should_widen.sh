#!/bin/sh
# Let the interpreter's machine-word add wrap instead of promoting to a
# bignum, which is what a promotion one step late looks like from outside:
# `big + 1` prints -9223372036854775808. Native refuses the same program, so
# only the one-engine corpus's an_int_past_int64_on_the_interpreter can see it.
set -e
old='            (Int::Small(a), Int::Small(b)) => match a.checked_add(*b) {'
[ "$(grep -cxF "$old" src/int.rs)" -eq 1 ]
sed -i.bak 's#^            (Int::Small(a), Int::Small(b)) => match a.checked_add(\*b) {$#            (Int::Small(a), Int::Small(b)) => match Some(a.wrapping_add(*b)) {#' src/int.rs
rm -f src/int.rs.bak
grep -qxF '            (Int::Small(a), Int::Small(b)) => match Some(a.wrapping_add(*b)) {' src/int.rs
