#!/bin/sh
# A string or literal cell is emitted only when the body or a type table names
# it. This mutation wants every one, so a program whose library functions were
# pruned carries their strings again. The output is the same; the witness is
# a_string_nothing_names_is_not_emitted, which finds strings nothing names.
set -e
line='        let wanted = |sym: &str| named.contains(sym) || tabled.contains(sym);'
grep -qF "$line" src/codegen.rs || {
  echo "the string filter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let wanted = |sym: &str| named.contains(sym) || tabled.contains(sym);$/        let wanted = |_: \&str| true;/' src/codegen.rs
[ "$(grep -cF '        let wanted = |_: &str| true;' src/codegen.rs)" -eq 1 ]
