#!/bin/sh
# The release link unswitches a loop on a test of a value the loop does not
# change. This mutation takes the pass out of the link's pipeline. The answers
# are the same; the work vein is the witness.
set -e
pass='simple-loop-unswitch<nontrivial;trivial>'
[ "$(grep -cF "$pass" src/main.rs)" -eq 1 ] || {
  echo "the link's unswitch pass moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/,simple-loop-unswitch<nontrivial;trivial>//' src/main.rs
if grep -qF "$pass" src/main.rs; then exit 1; fi
