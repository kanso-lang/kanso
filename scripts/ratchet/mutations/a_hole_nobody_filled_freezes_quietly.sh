#!/bin/sh
# The 2026-08-24 ruling: a hole is filled exactly once before its block
# freezes. This lets a block freeze over a hole nothing wrote, so
# a_hole_never_filled_before_the_freeze compiles and the errors corpus reads
# an empty stderr where the golden holds the refusal.
set -e
grep -qF 'if !hole.filled {' src/check.rs
sed -i 's/if !hole.filled {/if false {/' src/check.rs
! grep -qF 'if !hole.filled {' src/check.rs
