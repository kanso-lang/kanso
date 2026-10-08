#!/bin/sh
# The tab takes what its entry cannot reach out of the program, and a group
# the emitter asks about by name that the walk left out sends the program back
# to be emitted whole. This mutation stops noting the question, so a records
# `+` whose arm was left out goes down the numeric path unseen, and the unit
# spec that leaves the arm out says nothing saw it.
set -e
kept='        if self.left_out.contains(name) {'
[ "$(grep -cxF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the question moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if self\.left_out\.contains(name) {$/        if false \&\& self.left_out.contains(name) {/' src/codegen.rs
if grep -qxF "$kept" src/codegen.rs; then exit 1; fi
