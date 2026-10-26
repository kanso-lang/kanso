#!/bin/sh
# Let an in-place concat of any list literal cross a rewind by pointer.
#
# Strings made on the lap and concatenated into the accumulator are then
# left in arena memory the rewind reclaims, and the list reads them back
# overwritten.
set -e
sed -i.bak 's/scalar_elem(x, program, own, decl, inference, decl_index)/true || scalar_elem(x, program, own, decl, inference, decl_index)/' src/beat.rs
rm -f src/beat.rs.bak
test "$(grep -c 'true || scalar_elem(x, program, own, decl, inference, decl_index)' src/beat.rs)" = 1
