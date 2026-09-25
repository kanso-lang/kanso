#!/bin/sh
# A hole is filled exactly once. This lets a second write land on a hole the
# first already filled, so a_hole_filled_twice compiles and the errors corpus
# reads nothing where the golden holds the refusal.
set -e
grep -qF 'Some(hole) if hole.filled =>' src/check.rs
sed -i 's/Some(hole) if hole.filled =>/Some(hole) if false =>/' src/check.rs
! grep -qF 'Some(hole) if hole.filled =>' src/check.rs
