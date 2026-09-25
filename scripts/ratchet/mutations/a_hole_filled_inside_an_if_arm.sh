#!/bin/sh
# A write inside an `if` arm may not run, and a hole is filled exactly once.
# This lets the arm's write count as the fill, so
# a_birth_recorded_inside_an_if_arm loses its first diagnostic and the errors
# corpus reads two lines where the golden holds three.
set -e
grep -qF 'Some(_) if conditional =>' src/check.rs
sed -i 's/Some(_) if conditional =>/Some(_) if false =>/' src/check.rs
! grep -qF 'Some(_) if conditional =>' src/check.rs
