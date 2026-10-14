#!/bin/sh
# The pool forces each queued cell inside the try that catches a death, so a
# failure in pooled work is raised where the program reads the value. This
# mutation forces the cell bare: the overflow ends the program in the pool,
# before "first" prints.
set -e
line='        if (t->forced != K_MEMO_KEPT) k_ahead_guard(k_ahead_run_cell, &c, cells, n);'
[ "$(grep -cxF "$line" src/runtime/effects.c)" -eq 1 ] || {
  echo "the drain's guarded force moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if (t->forced != K_MEMO_KEPT) k_ahead_guard(k_ahead_run_cell, &c, cells, n);$/        if (t->forced != K_MEMO_KEPT) { (void)k_force(c); (void)cells; (void)n; }/' src/runtime/effects.c
if grep -qxF "$line" src/runtime/effects.c; then exit 1; fi
