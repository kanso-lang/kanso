#!/bin/sh
# Never take a write's value on the holder count alone, as before 2026-09-28.
#
# A loop seeded from a name its caller reads again copies its accumulator on
# every lap in the interpreter, and the allocations grow with the square of
# the laps.
set -e
sed -i.bak 's/|| (alone \&\& self.moved_here(span, frame))/|| (alone \&\& false \&\& self.moved_here(span, frame))/' src/eval.rs
rm -f src/eval.rs.bak
test "$(grep -c '(alone && false && self.moved_here(span, frame))' src/eval.rs)" = 3
