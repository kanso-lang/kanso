#!/bin/sh
# The json decoder answers a checksum that is not the one it owes.
#
# The end-to-end job builds bench/jsonbench natively and decodes
# bench/large.json a hundred and fifty times, summing the length of each result
# so the loop cannot be folded away. 24000 is a hundred and fifty times the
# hundred and sixty elements the document's top-level array holds, and the
# number is the only thing standing between a decoder that reads the document
# and one that reads something else.
#
# The array accumulator is where a wrong reading shows up in the length rather
# than in a value: pushing each element twice doubles every array in the tree,
# so the top level answers three hundred and twenty and the checksum answers
# 48000. The element is still used, so the tree still compiles.
#
# lib/json is what make_jsonbench copies into the benchmark, so patching the
# library reaches the built binary.
#
# 2026-09-06: this patched `acc2 = push acc v`, a binding that went when
# `array_step` stopped calling `skip_ws` and started handing its byte straight
# to `array_delim`. The push is an argument now and the mutation doubles it
# there. The ratchet caught the staleness on the same branch that caused it,
# which is what it is for.
set -e
f=lib/json/value.kso
grep -qF 'array_delim cs cs[p] p (push acc v)' "$f" || {
  echo "the array accumulator moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^  array_delim cs cs\[p\] p (push acc v)$|  array_delim cs cs[p] p (push (push acc v) v)|' "$f"
grep -qF 'array_delim cs cs[p] p (push (push acc v) v)' "$f"
