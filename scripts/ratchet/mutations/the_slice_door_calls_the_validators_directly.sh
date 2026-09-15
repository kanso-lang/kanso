#!/bin/sh
# k_b_utf8_slice_raw validates through k_utf8_bad_rare since 2026-09-15: the
# same ascii test, with the two validators behind preserve_most wrappers,
# because 98 of its 100 runs are ascii and the two that are not can pay the
# wrapper's saves rather than every call paying a frame. This sends it
# through the plain door again and the pushes return to all 861,498 calls.
# The answer is identical either way, so nothing but the work vein can see
# this, which is exactly what the row asserts.
set -e
grep -q "KValue bad = k_utf8_bad_rare(data, len, origin, NULL);" src/runtime.c || {
  echo "the slice door changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/KValue bad = k_utf8_bad_rare(data, len, origin, NULL);/KValue bad = k_utf8_bad(data, len, origin, NULL);/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
if grep -q "k_utf8_bad_rare(data, len, origin, NULL)" src/runtime.c; then
  echo "the rare door survived the mutation" >&2
  exit 1
fi
