#!/bin/sh
# The literal word's slow arm appends in place. This mutation sends it through
# the append that claims the room and hands back a new header, which a builder
# carried through a beat loop never sees: the mem corpus's counters drift by a
# header an append, and a_literal_appended_across_a_rewind crashes.
set -e
grep -q '^    return k_b_append_mut(acc, k_str_lit(lit, n, cell));$' src/runtime.c || {
  echo "the literal word's slow arm changed shape; rewrite this" >&2
  exit 1
}
sed -i 's/^    return k_b_append_mut(acc, k_str_lit(lit, n, cell));$/    return k_b_append(acc, k_str_lit(lit, n, cell));/' src/runtime.c
grep -q '^    return k_b_append(acc, k_str_lit(lit, n, cell));$' src/runtime.c
