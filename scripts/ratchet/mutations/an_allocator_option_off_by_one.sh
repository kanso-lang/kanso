#!/bin/sh
# Turn off the mimalloc option one past the one that matters.
#
# The compiler names `mi_option_arena_eager_commit` by a number copied out of
# the C library's header, and a copied number is exactly the kind that goes
# stale without saying so. This is what staleness looks like: the constant is
# still there, `mi_option_set` is still called, and the arena comes back.
set -e
sed -i.bak 's/^const ARENA_EAGER_COMMIT: i32 = 4;$/const ARENA_EAGER_COMMIT: i32 = 5;/' src/main.rs
rm -f src/main.rs.bak
grep -q '^const ARENA_EAGER_COMMIT: i32 = 5;$' src/main.rs
! grep -q '^const ARENA_EAGER_COMMIT: i32 = 4;$' src/main.rs
