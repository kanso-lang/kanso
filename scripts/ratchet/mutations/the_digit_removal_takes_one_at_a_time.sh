#!/bin/sh
# ryu_d2d strips the digits a shortest representation does not need, and since
# 2026-09-14 the general loop takes TWO a trip while two are there to take.
# Before that the hundred-step fired once and handed the rest to the ten-loop,
# which averaged 9.41 trips a float on the encode corpus. Both loops cost the
# same sixteen instructions a trip -- three multiply-highs, three shifts, a
# compare and the branch -- so a trip that takes two digits is worth two that
# take one.
#
# This puts the hundred-step back to firing once. The digits out are the same
# either way; the loop only decides how fast it gets to them. So nothing but
# the instruction veins can see this, which is exactly what the row asserts.
set -e
grep -q 'if (vpd100 <= vmd100) break;' src/runtime.c || {
  echo "the hundred-step loop changed shape; this mutation needs rewriting" >&2
  exit 1
}
awk '
  /^        for \(;;\) \{$/ && !fired { pending = 1; next }
  pending && /uint64_t vpd100 = vp \/ 100, vmd100 = vm \/ 100;$/ {
    print "        uint64_t vpd100 = vp / 100, vmd100 = vm / 100;"; next
  }
  pending && /if \(vpd100 <= vmd100\) break;$/ {
    print "        if (vpd100 > vmd100) {"; pending = 0; fired = 1; next
  }
  pending { print "        for (;;) {"; pending = 0; print; next }
  { print }
' src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q 'if (vpd100 > vmd100) {' src/runtime.c
