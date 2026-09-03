#!/bin/sh
# The one answer the per-chip compile row may never give.
#
# bench/compile_instructions_by_cpu.txt keys the front end's instruction count
# by silicon, because the runner pool is at least four CPUs and glibc picks its
# memcpy by ifunc at load time. The tempting shape — and the shape that was
# built once and killed by CI in two runs — is to notice the chip is not a
# recorded one and skip. Three runs in four land on a cpu that is not any given
# recorded one, so most real regressions would go through; and this harness's
# own mutations redden these gates, so on those runs its rows would go blind,
# which is the single thing it exists to prevent.
#
# So the refusal becomes a skip, and the spec has to say so.
set -e
row=scripts/gates/compile_ir_row.sh
before=$(grep -c '^  exit 1$' "$row")
if [ "$before" -ne 4 ]; then
  echo "expected four refusals in $row, found $before;" >&2
  echo "the shape moved and this mutation needs rewriting" >&2
  exit 1
fi
sed -i '/one per CI run\./,/^  exit 1$/ s/^  exit 1$/  exit 0/' "$row"
after=$(grep -c '^  exit 1$' "$row")
if [ "$after" -ne 3 ]; then
  echo "wanted exactly one refusal turned into a skip, $before became $after" >&2
  exit 1
fi
