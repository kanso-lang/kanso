#!/bin/sh
# A local arm shadows only the traveling arm of its own shape (2026-10-12,
# rule 4). This mutation keeps a call of a function the module declares off
# the traveling path, so a module that writes `area` for a square is handed a
# circle and finds no arm for it.
set -e
line="        (self.declared.fn_arities.declares(name) || self.declared.sibling_fns.contains(name))"
[ "$(grep -cxF "$line" src/check/shadow.rs)" -eq 1 ] || {
  echo "the shading test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s/^        (self.declared.fn_arities.declares(name) || /        false \&\& (self.declared.fn_arities.declares(name) || /" src/check/shadow.rs
grep -qF "        false && (self.declared.fn_arities.declares(name) || " src/check/shadow.rs
