#!/bin/sh
# The same file, the other arm. `die(format!(` does not carry the opening quote
# — the literal may sit on the next line — so the message is the SECOND piece
# of a split on the quote where the plain `die("` opener takes the first. A
# mutation on the plain arm alone would leave that difference untested.
#
# The bait is the map-key refusal, which a program cannot reach:
# `require_literal_key` admits only an int and a string with no interpolation,
# so a key that is neither is refused at compile time and never arrives here.
set -e
grep -qF 'die(format!("map keys are ints or strings, not {}"' src/wasm_rt.rs || {
  echo "the page's map-key refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|die(format!("map keys are ints or strings, not {}"|die(format!("this page refusal has no golden at all, not {}"|' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
grep -qF 'this page refusal has no golden at all' src/wasm_rt.rs
