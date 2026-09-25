#!/bin/sh
# A call's result proven to be one kind of value is not said to be one.
#
# Where a group's inferred result is exactly one kind, the call site hands
# LLVM that tag, which lets it fold the tests the caller's inlined helpers
# make on the result. With the call site's assume gone those tests come back
# and the run program's instruction row moves.
set -e
f=src/codegen.rs
grep -q 'fn assume_tag(&self' src/codegen.rs
line='            if callee_ret == "%KValue" && self.sub_parents.is_empty() {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the result assume's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            if callee_ret == "%KValue" \&\& self.sub_parents.is_empty() {$/            if false \&\& callee_ret == "%KValue" \&\& self.sub_parents.is_empty() {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
