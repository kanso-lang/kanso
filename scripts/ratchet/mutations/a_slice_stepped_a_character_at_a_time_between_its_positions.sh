#!/bin/sh
# text/slice of multibyte text walks from the front to the two character
# positions it needs, and a distance of ten characters or more between them is
# counted by k_slice_skip rather than stepped through character by character:
# in blocks of sixty-four bytes while the target is far off, then a word at a
# time by continuation bytes. runbench's index phase slices 690,000 characters
# out of 1.4 MB once. This mutation makes the skip return at once, so the
# character walk covers the whole distance again. The bytes out are the same
# and no counter moves, so the work vein is the witness.
set -e
target='static void k_slice_skip(KStr* s, long* at, long long* seen, long long next) {'
n=$(grep -cF "$target" src/runtime/text.c)
[ "$n" -eq 1 ] || { echo "k_slice_skip changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^\(static void k_slice_skip(KStr\* s, long\* at, long long\* seen, long long next) {\)$|\1 return;|' src/runtime/text.c
grep -qF 'long long next) { return;' src/runtime/text.c
