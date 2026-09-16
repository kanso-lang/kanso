#!/bin/sh
# A plain read handed to a group with no `none` arm is refused unless the
# `if`s around it prove the index in range (ruled 2026-09-16). `in_range` is
# the prover, and this mutation makes it answer no whatever facts are in
# force, so every guarded read in the tree goes back to being a none the
# checker refuses. The micro corpus reads a_guard_proves_the_read printing
# its elements; under the mutation the check refuses the program.
set -e
grep -q '^    if !lower {$' src/infer.rs || {
  echo "in_range's lower-bound test changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^    if !lower {$@    if lower || !lower {@' src/infer.rs
grep -q '^    if lower || !lower {$' src/infer.rs
