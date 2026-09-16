#!/bin/sh
# A name bound below a `return x if c` guard is a statement of the guard's
# `rest`, and the linearity analysis follows an accumulator through the names
# it is bound to. This puts the lookup back to the top level of the body: the
# guard's arm is never searched, `opened` in the fixture is not a binding the
# analysis can find, the accumulator handed on through it reads as an alias,
# and the loop below loses its in-place append and its beat.
set -e
grep -q '                if let Some(e) = bound_in(rest, var) {' src/linear.rs
sed -i 's|                if let Some(e) = bound_in(rest, var) {|                if let Some(e) = bound_in(\&[], var) {|' src/linear.rs
grep -q 'bound_in(&\[\], var)' src/linear.rs
