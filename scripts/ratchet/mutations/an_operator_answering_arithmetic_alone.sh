#!/bin/sh
# Inference reads `a + b` as arithmetic alone.
#
# A record on either side of an operator goes to the program's own arms for
# it, on every engine, so inference widens those arms' parameters and adds
# what they answer. With that gone, a function whose `+` meets a float or a
# record is inferred to answer a float, release assumes the float tag where a
# record comes back, and the micro fixture
# `an_operator_answers_what_its_arms_answer` prints its record through the
# number arm.
set -e
f=src/infer.rs
line='                Some(&(start, end, _)) if (a | b) & REC != 0 => {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the operator-arm match in eval_expr moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                Some(&(start, end, _)) if (a | b) \& REC != 0 => {$/                Some(\&(start, end, _)) if false \&\& (a | b) \& REC != 0 => {/' "$f"
grep -qxF '                Some(&(start, end, _)) if false && (a | b) & REC != 0 => {' "$f"
