#!/bin/sh
# A push, put or concat onto a field of a record that is handed over whole
# and read only through its fields writes into the field's value, when every
# record that reaches the parameter was built with a value nothing else holds
# there. This mutation answers no for every field read, so a list held in a
# record is copied on every push and the mem golden for it allocates
# gigabytes where it allocated two megabytes.
set -e
line='            return self.field_moves(p, f, ctx);'
[ "$(grep -cxF "$line" src/linear.rs)" -eq 1 ] || {
  echo "the field read test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            return self.field_moves(p, f, ctx);$/            return p.is_empty() \&\& self.field_moves(p, f, ctx);/' src/linear.rs
if grep -qxF "$line" src/linear.rs; then exit 1; fi
