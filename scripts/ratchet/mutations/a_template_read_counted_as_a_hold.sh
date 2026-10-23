#!/bin/sh
# An arm that only reads a parameter into templates leaves nothing holding it,
# so the parameter can still be an accumulator. This mutation counts those
# reads as holds again, so `grown`'s accumulator in the mem fixture is not
# unique, the record built from it is not alone in its field, and the field
# join copies. The output is the same; the mem vein's
# `a_record_field_joined_in_place` allocates a copy a pass.
set -e
line='                    if effective_uses(pname, &decl.body) > 1 && !only_rendered(pname, &decl.body) {'
grep -qxF "$line" src/linear.rs || {
  echo "the template-read exemption moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    if effective_uses(pname, \&decl.body) > 1 \&\& !only_rendered(pname, \&decl.body) {$/                    if effective_uses(pname, \&decl.body) > 1 \&\& (true || !only_rendered(pname, \&decl.body)) {/' src/linear.rs
if grep -qxF "$line" src/linear.rs; then exit 1; fi
