#!/bin/sh
# The release link runs correlated propagation after each jump-threading. This
# mutation takes both out of the link's pipeline. The answers are the same;
# the work vein is the witness.
set -e
[ "$(grep -o 'correlated-propagation' src/main.rs | wc -l)" -ge 2 ] || {
  echo "the link's correlated propagation moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/jump-threading,correlated-propagation/jump-threading/g' src/main.rs
if grep -q 'jump-threading,correlated-propagation' src/main.rs; then exit 1; fi
