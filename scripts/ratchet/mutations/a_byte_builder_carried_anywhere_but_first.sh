#!/bin/sh
# The beat's boundary rule licenses a byte builder to cross a rewind when the
# self-call's argument chains from the builder that arrived, and since
# 2026-09-16 it asks that of the parameter at the position under test as well
# as the first one. This puts the first-parameter-only reading back: a
# builder carried anywhere else reads as heap the loop may not rewind over,
# and builder_transient's `assemble cs p acc` loses its beat (1,360 -> 40).
set -e
grep -q '    for slot in \[position, 0\] {' src/beat.rs
sed -i 's|    for slot in \[position, 0\] {|    for slot in [0] {|' src/beat.rs
grep -q '    for slot in \[0\] {' src/beat.rs
