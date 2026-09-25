#!/bin/sh
# The spelling the ruling retired: `none` as a placeholder for a field the
# block writes later. This lets a write land on a field built with a value,
# so a_field_written_that_was_not_left_as_a_hole compiles and the errors
# corpus reads nothing where the golden holds the refusal.
set -e
grep -qF '        let message = match hole {' src/check.rs
sed -i 's/        let message = match hole {/        if hole.is_none() \&\& !chosen { return; }\n        let message = match hole {/' src/check.rs
grep -qF 'if hole.is_none() && !chosen { return; }' src/check.rs
