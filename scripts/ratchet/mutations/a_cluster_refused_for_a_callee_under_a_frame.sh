#!/bin/sh
# A cluster entered by a tail call is refused its bracket when the cluster
# can reach the entering caller with no frame held. This mutation counts
# every mention again, plain calls included, so a loop that runs its body
# through a plain call to the caller that entered it loses its beat and the
# mem golden for it grows to eleven arena blocks.
set -e
line='            frameless_names(guard_stmt_expr(stmt), entry);'
[ "$(grep -cxF "$line" src/beat.rs)" -eq 1 ] || {
  echo "the frameless mention walk moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            frameless_names(guard_stmt_expr(stmt), entry);$/            collect_names(guard_stmt_expr(stmt), entry);/' src/beat.rs
if grep -qxF "$line" src/beat.rs; then exit 1; fi
