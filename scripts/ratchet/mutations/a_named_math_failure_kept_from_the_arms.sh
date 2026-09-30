#!/bin/sh
# Native types a division's answer as a number or a failure even where the
# program declares `divide_by_zero`.
#
# An operator asks its user arms only when a side may be a record, and native
# reads that from the set it recorded for the operand. Where the program names
# `math_failure`, a zero divisor answers a subtype, which the arms have to be
# asked about, so a division's answer carries the record bit there. Without
# it, native's `+` never asks, and the micro fixture is refused where the
# interpreter runs the arm.
set -e
f=src/codegen.rs
grep -q "fn zero_divisor_set(&self) -> Set" src/codegen.rs
line='            true => REC,'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the zero-divisor set moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            true => REC,$/            true => 0,/' "$f"
grep -qxF '            true => 0,' "$f"
