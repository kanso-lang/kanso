#!/bin/sh
# Round a finite float with a saturating cast, as the interpreter did before
# 2026-09-28.
#
# 1e30 rounds to 9223372036854775807 rather than the thirty-one digit integer
# it is, and the spec's interpreted reading of it fails.
set -e
sed -i.bak 's|^                    Value::Float(v) if v.is_finite() => {$|                    Value::Float(v) if false \&\& v.is_finite() => {|' src/eval.rs
rm -f src/eval.rs.bak
grep -q 'Value::Float(v) if false && v.is_finite() => {' src/eval.rs
