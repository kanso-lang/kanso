#!/bin/sh
# Take whatever last digit Rust's `{:e}` chose, as the interpreter did before
# 2026-09-28.
#
# A double halfway between two shortest decimals prints its upper candidate
# interpreted and the even one natively, and the micro fixture
# a_float_halfway_between_two_shortest_takes_the_even_digit disagrees.
set -e
grep -q '^    if last.is_multiple_of(2) {$' src/eval.rs
sed -i.bak 's/^    if last.is_multiple_of(2) {$/    if last.is_multiple_of(1) {/' src/eval.rs
rm -f src/eval.rs.bak
grep -q '^    if last.is_multiple_of(1) {$' src/eval.rs
