#!/bin/sh
# text/slice of multibyte text walks from the front to the two character
# positions it needs, and since 2026-09-07 a distance of ten characters or
# more between them is counted a word at a time by its continuation bytes
# rather than stepped through character by character: runbench's index
# phase slices 690,000 characters out of 1.4 MB once. This mutation makes
# the word skip count nothing, so the character walk covers the whole
# distance again. The bytes out are the same and no counter moves, so the
# work vein is the witness.
set -e
target='static void k_slice_skip(KStr* s, long* at, long long* seen, long long next) {'
n=$(grep -cF "$target" src/runtime.c)
[ "$n" -eq 1 ] || { echo "k_slice_skip changed shape ($n); rewrite this" >&2; exit 1; }
sed -i '/^static void k_slice_skip(KStr\* s, long\* at, long long\* seen, long long next) {$/{n;s|^    while (\*at + 8 <= s->len) {$|    while (0) {|}' src/runtime.c
grep -qF '    while (0) {' src/runtime.c
