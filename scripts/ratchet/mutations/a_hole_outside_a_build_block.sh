#!/bin/sh
# `_` stands only where a construction's argument goes, inside a `build`
# block. This lets it stand anywhere, so a_hole_outside_a_build_block
# compiles as far as the checker is concerned and the errors corpus reads
# something other than the two refusals its golden holds.
set -e
grep -qF 'if !flags.hole_ok {' src/check.rs
sed -i 's/if !flags.hole_ok {/if false {/' src/check.rs
! grep -qF 'if !flags.hole_ok {' src/check.rs
