#!/bin/sh
# A slice of sixty-four bytes or more shares its string's bytes: a header that
# points into the parent. This mutation raises the threshold past any string,
# so every slice is a copy again. The characters are the same; the witness is
# the fixture a_long_slice_shares_its_text, whose alloc_bytes counts each
# slice's bytes a second time.
set -e
grep -qF '#define K_STR_VIEW_MIN 64' src/runtime.c || {
  echo "the view threshold moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^#define K_STR_VIEW_MIN 64$|#define K_STR_VIEW_MIN (1LL << 40)|' src/runtime.c
[ "$(grep -cF '#define K_STR_VIEW_MIN (1LL << 40)' src/runtime.c)" -eq 1 ]
