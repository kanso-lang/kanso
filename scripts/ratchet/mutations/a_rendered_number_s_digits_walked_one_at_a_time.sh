#!/bin/sh
# ryū writes a number's significant digits into a stack array and render_ryu
# copies the run it needs into the output buffer. Written as a byte loop,
# clang outlined all three of them into calls to memcpy -- 191,070 a run on
# runbench at 15.3 instructions apiece, where the digits being moved are a
# handful. Since 2026-09-08 they go through `k_copy_short`. This mutation puts
# the byte loops back, which is the shape clang turns into the call again.
set -e
for t in '            k_copy_short(o, dig + 1, k - 1); o += k - 1;' \
         '            k_copy_short(o, dig + ip, k - ip); o += k - ip;' \
         '        k_copy_short(o, dig, k); o += k;'; do
  n=$(grep -cF "$t" src/runtime.c)
  [ "$n" -eq 1 ] || { echo "render_ryu's digit copies changed shape ($n); rewrite this" >&2; exit 1; }
done
sed -i 's|^            k_copy_short(o, dig + 1, k - 1); o += k - 1;$|            for (int i = 1; i < k; i++) *o++ = dig[i];|; s|^            k_copy_short(o, dig + ip, k - ip); o += k - ip;$|            for (int i = ip; i < k; i++) *o++ = dig[i];|; s|^        k_copy_short(o, dig, k); o += k;$|        for (int i = 0; i < k; i++) *o++ = dig[i];|' src/runtime.c
grep -qF '        for (int i = 0; i < k; i++) *o++ = dig[i];' src/runtime.c
