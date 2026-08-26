#!/bin/sh
# The error corpus exists to prove every diagnostic the language emits is still
# emitted. A check that quietly stops firing is the regression it is for, so
# that is what this introduces: the none-in-a-list walk still runs and still
# looks at every list, and sees nothing in any of them.
set -e
sed -i 's/for item in items\.iter()\.filter(|i| is_none_lit(i))/for item in items.iter().take(0).filter(|i| is_none_lit(i))/' src/check.rs
grep -q 'take(0)' src/check.rs
