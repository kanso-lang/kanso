#!/bin/sh
# Rank floats by bit pattern again, as f64::total_cmp did before the
# 2026-09-30 float order: `inf - inf` carries the sign bit on x86 and sorts
# below -inf, and -0.0 falls below 0.0. The interpreter then parts from native
# on every_nan_is_one_value_and_zero_is_one_value.
set -e
old='        (Value::Float(x), Value::Float(y)) => Some(float_order(*x, *y)),'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^        (Value::Float(x), Value::Float(y)) => Some(float_order(\*x, \*y)),$#        (Value::Float(x), Value::Float(y)) => Some(x.total_cmp(y)),#' src/eval.rs
rm -f src/eval.rs.bak
grep -qxF '        (Value::Float(x), Value::Float(y)) => Some(x.total_cmp(y)),' src/eval.rs
