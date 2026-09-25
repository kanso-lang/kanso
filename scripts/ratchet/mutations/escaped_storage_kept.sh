#!/bin/sh
# The registration that gives an escaped accumulator's storage an owner goes
# away, so the last buffer of every such list is held to exit. Measured on the
# escape benchmark: perm_live_bytes 0 becomes 24,624,000 and bytes_freed 3,000
# becomes 0. This is the leak that took vse's peak from flat to doubling with
# its trial count.
#
# It removes the registration rather than editing the golden because the golden
# is what an edit would prove nothing about. A counter reading a constant, or
# one wired to a pool nobody allocates from, survives an edited golden and dies
# here.
set -e
A='        if (perm) k_permreg_add(l, &l->items);' \
awk '
  !hit && $0 == ENVIRON["A"] { hit = 1; next }
  { print }
  END { if (!hit) exit 3 }
' src/runtime.c > src/runtime.c.mut || {
  rm -f src/runtime.c.mut
  echo "the list growth registration moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/runtime.c.mut src/runtime.c
grep -q 'k_permreg_add(l, &l->items)' src/runtime.c && exit 1
exit 0
