#!/bin/sh
# Let an arm keep an element of a map whose values are promised alone.
#
# A helper that stores `m[0]` in a list inside the map, run between two
# counts, then sees the next count written into the map it stored.
set -e
sed -i.bak 's/(kept == Kept::Read || take_of(p, e, \&ctx.body).is_some())/(true || take_of(p, e, \&ctx.body).is_some())/' src/linear.rs
rm -f src/linear.rs.bak
test "$(grep -c '(true || take_of(p, e, &ctx.body).is_some())' src/linear.rs)" = 1
