#!/bin/sh
# The same rule as a_diagnostic_arrives_without_a_golden, on the page's opener.
#
# `src/wasm_rt.rs` writes 36 `die(` sites spelling 24 sentences, and until the
# scan learned this fifth opener it walked past every one of them — on the
# engine a reader meets first, since the website's playground runs it. A page
# refusal could be reworded or lost with nothing going red, and three of them
# turned out this week to be saying the wrong thing entirely.
#
# The bait is the builtin-name refusal, which the string-literal guarantee puts
# out of a program's reach: every literal position in an emitted module is
# written by `str_lit`, so the handle this arm inspects holds a string by
# construction. Excused today; a novel sentence in its place is not.
set -e
grep -qF 'die("builtin name must be a string".to_string())' src/wasm_rt.rs || {
  echo "the page's builtin-name refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|die("builtin name must be a string".to_string())|die("this page refusal has no golden anywhere".to_string())|' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
grep -qF 'this page refusal has no golden anywhere' src/wasm_rt.rs
