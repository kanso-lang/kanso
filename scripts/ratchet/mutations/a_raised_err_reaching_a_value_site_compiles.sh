#!/bin/sh
# A failure is a box (#2087): a group whose every answer is a failure answers
# a box, and so does a tail that calls `err`, and a box where a value is read
# does not compile. Either test alone refuses `closed 1 + 1`, so this drops
# both, and the error fixture a_failure_where_a_value_is_expected compiles
# clean where its golden holds the refusal.
set -e
f=src/check/after_infer.rs
grep -q 'let boxed = |s: Set|' src/check/after_infer.rs
set_line='    let boxed = |s: Set| s & (DESC | FAIL) != 0 && s & values == 0;'
head_line='                        if name == "effect" || name == "print" || name == "err") =>'
[ "$(grep -cxF "$set_line" "$f")" -eq 1 ] && [ "$(grep -cxF "$head_line" "$f")" -eq 1 ] || {
  echo "the box test changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let boxed = |s: Set| s & (DESC | FAIL) != 0 && s & values == 0;$/    let boxed = |s: Set| s \& DESC != 0 \&\& s \& values == 0;/' "$f"
sed -i 's/^                        if name == "effect" || name == "print" || name == "err") =>$/                        if name == "effect" || name == "print") =>/' "$f"
if grep -qxF "$set_line" "$f" || grep -qxF "$head_line" "$f"; then exit 1; fi
