#!/bin/sh
# The wrapper of a function value passes a record in registers straight on.
#
# A function value is called through a wrapper that answers one boxed word. A
# group that answers its record in registers is called that way by the wrapper
# too, which builds the record from the three words. With the wrapper calling
# every group as though it answered a boxed word, a handed-out decoder's
# caller reads the first register as a record.
set -e
f=src/codegen.rs
grep -q "fn simple_fn_value" "$f"
grep -qF '                .filter(|_| self.ret_ty(name, arity) == "%parsed")' "$f" || {
  echo "the wrapper's boxing of a by-value answer moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                \.filter(|_| self\.ret_ty(name, arity) == "%parsed")$/                .filter(|_| false \&\& self.ret_ty(name, arity) == "%parsed")/' "$f"
grep -qF '.filter(|_| false && self.ret_ty(name, arity) == "%parsed")' "$f"
