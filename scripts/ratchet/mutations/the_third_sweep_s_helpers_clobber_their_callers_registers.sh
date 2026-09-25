#!/bin/sh
# The third cold-frame sweep, 2026-09-15: nine more rare arms of hot runtime
# functions are preserve_most helpers -- the character scan behind an index
# and a slice, a map view's first build, `entries`' failing-field record,
# a list grow's permanent buffer, its registration and its release, a bytes
# grow's malloc regime and its release, and the tenure tier's block opener.
# A caller of a preserve_most function keeps its live values in caller-saved
# registers across the call, so k_b_at, k_b_entries, k_b_push_grow,
# k_b_append_grow, k_closure and k_copy_alloc open with one push or none
# where they opened with five to seven.
#
# This strips the attribute from the nine, and from nothing else: the six
# kanso#1429 marked keep theirs. Every value out is identical, so nothing
# but the work vein can see it, which is exactly what the row asserts.
set -e
for n in k_str_chars_scan k_map_sort_build k_rec_cold k_buf_perm k_permreg_add \
         k_buf_release k_bytes_buf_malloc k_bytes_buf_release k_ten_block_open; do
  grep -q "preserve_most)) [^(]* $n(" src/runtime.c || {
    echo "$n's attributes changed shape; this mutation needs rewriting" >&2
    exit 1
  }
  sed -e "/preserve_most)) [^(]* $n(/s/, preserve_most))/))/" src/runtime.c > src/runtime.c.mut
  mv src/runtime.c.mut src/runtime.c
done
for n in k_str_chars_scan k_map_sort_build k_rec_cold k_buf_perm k_permreg_add \
         k_buf_release k_bytes_buf_malloc k_bytes_buf_release k_ten_block_open; do
  if grep -q "preserve_most)) [^(]* $n(" src/runtime.c; then
    echo "$n kept preserve_most through the mutation" >&2
    exit 1
  fi
done
