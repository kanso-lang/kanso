#!/bin/sh
# A view ends inside its parent, so the byte after it belongs to the parent,
# and `k_cstr` copies it with a terminator before the C library reads it. This
# mutation hands the parent's bytes over as they are. The witness is the
# runtime fixture a_long_slice_ends_where_it_was_cut, whose sentence then runs
# on past the slice into the rest of the line.
set -e
grep -qF '    if (s->data[s->len] == 0) return s->data;' src/runtime.c || {
  echo "k_cstr moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    if (s->data\[s->len\] == 0) return s->data;$|    return s->data;|' src/runtime.c
[ "$(grep -cF '    return s->data;' src/runtime.c)" -eq 1 ]
