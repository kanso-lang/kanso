#!/bin/sh
# A group handed out as a function value may return its record in registers.
#
# A function value is called through a wrapper that answers one boxed word, so
# native refuses to wrap a group whose record comes back in two registers. With
# the check gone, the escape analysis lets such a group go by value and the
# build of a program that holds it with `&` or passes it by name is refused.
set -e
f=src/escape.rs
grep -q "fn value_names" src/escape.rs
grep -qF '        if self.returns_ty.iter().any(|(name, arity)| *arity > 0 && values.contains(*name)) {' "$f" || {
  echo "the value check in returnable moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if self.returns_ty.iter().any(|(name, arity)| \*arity > 0 \&\& values.contains(\*name)) {$/        if false \&\& self.returns_ty.iter().any(|(name, arity)| *arity > 0 \&\& values.contains(*name)) {/' "$f"
grep -qF 'if false && self.returns_ty.iter().any(' "$f"
