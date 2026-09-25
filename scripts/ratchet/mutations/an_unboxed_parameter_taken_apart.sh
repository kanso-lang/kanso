#!/bin/sh
# An unboxed parameter's words are recorded when it is boxed on entry, so a
# read of its tag or payload is the raw argument. This mutation records
# nothing, so every read takes the box back apart. The output is the same;
# the witness is an_unboxed_parameter_is_read_as_its_word.
set -e
line='                f.known_words.insert(format!("%x{i}"), ("0".to_string(), format!("%x{i}r")));'
grep -qF "$line" src/codegen.rs || {
  echo "the unboxed parameter's record moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^                f.known_words.insert(format!("%x{i}"), ("0".to_string(), format!("%x{i}r")));$/d' src/codegen.rs
[ "$(grep -cF 'f.known_words.insert(' src/codegen.rs)" -eq 0 ]
