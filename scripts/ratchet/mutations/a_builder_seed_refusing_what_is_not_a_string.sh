#!/bin/sh
# Native refuses a string builder's seed that is not a string.
#
# `"{s}x"` renders whatever `s` holds, so a group that joins onto its own
# parameter may be handed an int, a record or a subtype of either.
# `k_b_str_builder` hands such a seed on as it came and the first join adopts
# its rendering; dying there instead prints `a string builder starts from a
# string` where the interpreter prints the rendering.
set -e
f=src/runtime.c
grep -q 'KValue k_b_str_builder' src/runtime.c
grep -qF '    if (wrapped.tag != K_STR) return sv;' "$f" || {
  echo "the seed's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (wrapped.tag != K_STR) return sv;$/    if (wrapped.tag != K_STR) k_die("a string builder starts from a string");/' "$f"
grep -qF 'if (wrapped.tag != K_STR) k_die(' "$f"
