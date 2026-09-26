#!/bin/sh
# A carried record's whole word is asked about failure before its fields.
#
# A failure in a `%parsed` reads as one in the value field's tag byte, so a
# pattern whose value field refuses failures needs no test of the whole word
# first. Put that test back for every record and it is asked twice.
set -e
f=src/codegen.rs
grep -q 'fn emit_parsed_pattern(' src/codegen.rs
line='                if !value_refuses {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the carried record's failure test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                if !value_refuses {$/                if true {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
