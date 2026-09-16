#!/bin/sh
# Name the purge delay by the wrong number.
#
# The compiler sets `mi_option_purge_delay` to -1 so mimalloc never returns
# pages to the operating system on a timer — not to save the purging, but to
# stop it reading the clock. `_mi_clock_now` calls glibc's `clock_gettime`,
# which goes through the vDSO, and the vDSO's instruction count is a property
# of the host's clocksource rather than of this program. CI found that the
# hard way: the three compile rows came back 13 instructions apart on two runs
# of one commit, the same 13 on all three, on the same CPU model and the same
# glibc.
#
# A number copied out of a header goes stale in silence. This is what that
# looks like — the constant is still there, `mi_option_set` is still called,
# and the clock is back in the row.
set -e
sed -i.bak 's/^const PURGE_DELAY: i32 = 15;$/const PURGE_DELAY: i32 = 16;/' src/main.rs
rm -f src/main.rs.bak
grep -q '^const PURGE_DELAY: i32 = 16;$' src/main.rs
! grep -q '^const PURGE_DELAY: i32 = 15;$' src/main.rs
