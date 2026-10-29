#!/bin/sh
# A short float's text is written right to left in one pass over m: its p
# fraction digits, the point, then the whole part, with no division by 10^p
# to split them. This mutation puts the split back: the whole part divided
# out and written forward, then the fraction behind the point. Same text, and
# the run program reads heavier.
set -e
t='            char tmp[48];'
n=$(grep -cxF "$t" src/runtime/ryu.c)
[ "$n" -eq 1 ] || { echo "the short float's writer changed shape ($n); rewrite this" >&2; exit 1; }
awk -v t="$t" '
  $0 == t {
    print "            uint64_t whole = m / pw, part = m - whole * pw;"
    print "            char* o = buf + ryu_write(whole, buf);"
    print "            if (sp) {"
    print "                *o++ = (char)46;"
    print "                char* w = o + sp;"
    print "                o = w;"
    print "                int left = sp;"
    print "                while (left >= 2) {"
    print "                    uint32_t c = (uint32_t)(part % 100);"
    print "                    part /= 100;"
    print "                    w -= 2;"
    print "                    __builtin_memcpy(w, RYU_DIGITS + c * 2, 2);"
    print "                    left -= 2;"
    print "                }"
    print "                if (left) *--w = (char)(48 + (uint32_t)part);"
    print "            }"
    print "            *o = 0;"
    print "            return (long long)(o - buf);"
    skip = 1
    next
  }
  skip && $0 == "            return n;" { skip = 0; next }
  skip { next }
  { print }' src/runtime/ryu.c > src/runtime/ryu.c.new
mv src/runtime/ryu.c.new src/runtime/ryu.c
test "$(grep -cxF "$t" src/runtime/ryu.c)" = 0
grep -qxF '            uint64_t whole = m / pw, part = m - whole * pw;' src/runtime/ryu.c
