#!/bin/sh
# ryū's digit core ends by deciding whether the digits it kept round up:
# `output = vr + (vr == vm || round_up)`. Drop that decision and every
# rendering truncates instead. The text still looks like a number and still
# has the right length; it reads back as a DIFFERENT double.
#
# Nothing in the tree could see this before 2026-09-14. The engine
# differential compares two implementations and would keep passing if both
# truncated; the digit-extraction sweep checks the block that writes digits,
# not which digits were chosen. The round-trip harness is what catches it:
# 815,943 of 2,809,326 values stop reading back as themselves.
set -e
t='        output = vr + (vr == vm || round_up);'
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "ryu_d2d's rounding changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's#^        output = vr + (vr == vm || round_up);$#        output = vr;#' src/runtime.c
grep -qF '        output = vr;' src/runtime.c
