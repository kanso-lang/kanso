#!/bin/sh
# A guard's condition is built as a value and handed to k_truthy.
#
# A guard is a tail `if` whose else is the rest of the body, so its condition
# goes through emit_cond and a comparison or an `or` branches on its own i1.
# Put back the value form it had before 2026-09-26 and the guard builds a
# tagged boolean through a phi and asks the runtime whether it is true.
set -e
f=src/codegen.rs
grep -q 'fn emit_tail(&mut self' src/codegen.rs
line='            self.emit_cond(f, cond, &early_label, &rest_label, None)?;'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the guard's condition moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            self.emit_cond(f, cond, &early_label, &rest_label, None)?;$/            let v = self.emit_expr(f, cond)?;\n            let v = self.maybe_force(f, v);\n            self.test_cond_value(f, v, \&early_label, \&rest_label, None);/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
