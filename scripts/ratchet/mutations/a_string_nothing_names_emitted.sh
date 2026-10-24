#!/bin/sh
# A string or literal cell is emitted only when the body or a type table names
# it. This mutation starts every one named, so a program whose library
# functions were pruned carries their strings again. The output is the same;
# the witness is a_string_nothing_names_is_not_emitted, which finds strings
# nothing names.
set -e
line='        let mut named = vec![[false; 2]; self.strings.len()];'
[ "$(grep -cxF "$line" src/codegen/module.rs)" -eq 1 ] || {
  echo "the string filter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let mut named = vec!\[\[false; 2\]; self.strings.len()\];$/        let mut named = vec![[true; 2]; self.strings.len()];/' src/codegen/module.rs
[ "$(grep -cxF '        let mut named = vec![[true; 2]; self.strings.len()];' src/codegen/module.rs)" -eq 1 ]
