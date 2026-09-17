#!/bin/sh
# The build tree counted as one process.
#
# The codegen rows exist because `kanso check` stops before codegen, and what
# they count is the whole tree: kanso, the clang driver, `clang -cc1`, ld and
# the linker's own child. `--trace-children=yes` is the whole of how that
# happens, and dropping it leaves the compiler's own process, which is the
# smaller half of a release build. The gate refuses a tree shorter than four
# processes for the same reason, because the first measurement of this row was
# taken on a build that had been REFUSED after emitting its IR: two processes,
# both tiers within 0.006% of each other, entirely plausible and entirely wrong.
#
# So the mutation is the one line that makes the difference.
set -e
gate=scripts/gates/codegen_instructions.sh
before=$(grep -c -- '--trace-children=yes' "$gate" || true)
if [ "$before" -lt 2 ]; then
  echo "expected two --trace-children runs in $gate and found $before;" >&2
  echo "the gate's shape moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i 's/--trace-children=yes //g' "$gate"
if grep -q -- '--trace-children' "$gate"; then
  echo "wanted --trace-children gone from $gate, and it is still there" >&2
  exit 1
fi
