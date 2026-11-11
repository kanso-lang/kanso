#!/bin/sh
# k_ten_release reads one byte of a tenure block it has just freed. The byte
# is thrown away, so no program prints anything different and every golden
# still agrees; a sanitizer is the only reader that sees it (kanso#2449).
set -e
grep -c '^        free(b->data);$' src/runtime/core.c | grep -qx 1 || {
  echo "the free in k_ten_release moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^        free(b->data);$|&\n        { volatile char probe = ((const char*)b->data)[0]; (void)probe; }|' src/runtime/core.c
grep -q 'volatile char probe = ((const char\*)b->data)\[0\]' src/runtime/core.c
