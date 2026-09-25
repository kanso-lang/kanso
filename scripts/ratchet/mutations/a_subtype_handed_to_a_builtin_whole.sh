#!/bin/sh
# A builtin is handed a subtype's wrapper instead of the value it wraps.
#
# In a program that declares a subtype, the emitter routes every builtin's
# arguments through `k_unsub`, as the oracle's call_builtin unwraps them. With
# the routing gone, `length` of a `type name string` refuses the wrapper on
# the native engines while the interpreter counts the string, and the micro
# corpus reads the two engines apart.
set -e
f=src/codegen.rs
grep -q 'k_unsub(%KValue {e})' src/codegen.rs
line='        if !shadows && !self.sub_parents.is_empty() && crate::check::builtin_arity(name).is_some() {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the builtin unwrap's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if !shadows \&\& !self.sub_parents.is_empty() \&\& crate::check::builtin_arity(name).is_some() {$/        if false \&\& !shadows \&\& crate::check::builtin_arity(name).is_some() {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
