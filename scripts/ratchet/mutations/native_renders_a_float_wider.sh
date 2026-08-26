#!/bin/sh
# An integral float printed with two decimals instead of one, in the C runtime
# only. `1.0` reads as `1.00` on native and `1.0` on the oracle.
#
# Rendering is written twice — `render` in src/eval.rs and `k_render` in
# src/runtime.c — and every `print` goes through the second one, so this is the
# widest surface in the compiler where two implementations have to agree
# character for character. Only the render sweep asks both of them for the
# values nobody would think to print; the corpus pins the ones it happens to.
set -e
grep -q '"%.1f"' src/runtime.c || {
  echo "the integral-float format moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's/"%\.1f"/"%.2f"/' src/runtime.c
rm -f src/runtime.c.bak
grep -q '"%.2f"' src/runtime.c
