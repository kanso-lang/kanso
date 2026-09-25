#!/bin/sh
# k_viewreg_migrate, k_permreg_migrate and k_permreg_flush are inlined into
# k_beat_iter, and they stay that way only because they say so. Shrinking
# k_beat_rewind's fast path changed the inliner's budget in the caller and both
# migrate wrappers went out of line: encodebench rose 50,161,245 instructions
# where the change had saved 13,079,983 on escapebench. The pins are what make
# the summary word a win rather than a 0.32% regression, so removing them has
# to be visible.
#
# Nothing else in the tree can see this. The counter gates stay byte-identical
# either way — no statistic moves when a call is inlined or not — and the
# emitted golden counts what the compiler wrote, not what llvm did with it.
# The work vein is the only witness.
set -e
before=$(grep -c 'always_inline' src/runtime.c)
sed -i.bak 's/^static inline __attribute__((always_inline)) void k_\(viewreg_migrate\|permreg_migrate\|permreg_flush\)(int d) {$/static inline void k_\1(int d) {/' \
  src/runtime.c
rm -f src/runtime.c.bak
# k_alloc and k_str_alloc carry the same attribute for their own reasons and
# must not be touched, so this counts the three rather than asserting none are
# left. A sed that matched two of three would leave a mutation proving less
# than it claims, which is how a row goes quietly blind.
after=$(grep -c 'always_inline' src/runtime.c)
[ "$((before - after))" = "3" ]
grep -q '^static inline void k_viewreg_migrate(int d) {$' src/runtime.c
grep -q '^static inline void k_permreg_migrate(int d) {$' src/runtime.c
grep -q '^static inline void k_permreg_flush(int d) {$' src/runtime.c
