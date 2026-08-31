#!/bin/sh
# An integral float rendered ten times over, in the C runtime only. `1.0` reads
# as `10.0` on native and `1.0` on the oracle.
#
# Rendering is written twice — `render` in src/eval.rs and `k_render` in
# src/runtime.c — and every `print` goes through the second one, so this is the
# widest surface in the compiler where two implementations have to agree
# character for character. Only the render sweep asks both of them for the
# values nobody would think to print; the corpus pins the ones it happens to.
#
# This used to widen `"%.1f"` to `"%.2f"`. That format is gone: an integral
# double under 1e15 casts exactly, so the branch writes the integer's digits
# and a literal `.0` rather than calling into glibc's multiprecision
# formatter. The cast is the line to break now.
set -e
grep -qF 'long long whole = (long long)d;' src/runtime.c || {
  echo "the integral-float cast moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|long long whole = (long long)d;|long long whole = (long long)d * 10;|' src/runtime.c
grep -qF 'long long whole = (long long)d * 10;' src/runtime.c
