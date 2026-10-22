#!/bin/sh
# Where a map's sorted view is built, the encoder reads an entry's key and value
# with one test, written inline by k_b_entry_pair_fast: the view, the bounds,
# and two loads. This mutation sends every pair to the C, which answers the
# same key and value from two calls. The witness is the work vein: every
# program that encodes an object reads its entries the long way.
#
# It patched `key_at_fast` in emit.rs until 2026-10-22. The entry pair had by
# then taken over every key the encoder reads, so that patch moved nothing a
# benchmark runs, and the row read BLIND.
set -e
line='  br i1 %both, label %view, label %slow'
n=$(sed -n '/@k_b_entry_pair_fast(/,/^}/p' src/codegen/declares.rs | grep -cxF "$line")
[ "$n" -eq 1 ] || {
  echo "the entry pair's test moved ($n); this mutation needs rewriting" >&2
  exit 1
}
sed -i '/@k_b_entry_pair_fast(/,/^}/ s/^  br i1 %both, label %view, label %slow$/  br label %slow/' src/codegen/declares.rs
[ "$(sed -n '/@k_b_entry_pair_fast(/,/^}/p' src/codegen/declares.rs | grep -cxF '  br label %slow')" -eq 1 ]
