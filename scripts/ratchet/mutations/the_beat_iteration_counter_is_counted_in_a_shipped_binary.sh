#!/bin/sh
# Since 2026-09-15 every counter increment in src/runtime.c sits behind
# K_COUNTING, which is 0 in a shipped binary and 1 in a counted one, so the
# forty sites that used to increment unconditionally are gone from what a
# program runs: 6,425,360 increments a run on the run program, and the
# register traffic around them. This puts the beat iteration counter's two
# sites back unguarded. A counted binary counts the same either way, so
# every golden holds and the work vein is the only thing that can see it,
# which is what the row asserts.
set -e
n=$(grep -c "^    if (K_COUNTING) k_stat_beat_iters++;" src/runtime.c || true)
[ "$n" = "2" ] || {
  echo "the beat iteration counter's sites changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/^    if (K_COUNTING) k_stat_beat_iters++;/    k_stat_beat_iters++;/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "^    k_stat_beat_iters++;" src/runtime.c
