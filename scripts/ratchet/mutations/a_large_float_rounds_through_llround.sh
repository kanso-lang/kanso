#!/bin/sh
# Round a float with llround in the compiled runtime, as it did before
# 2026-09-28.
#
# llround answers LLONG_MIN for every float it cannot hold, so a compiled build
# prints -9223372036854775808 for round of 1e30 and of infinity where it should
# refuse the first and agree with the interpreter on the second.
set -e
sed -i.bak 's|^        double x = k_as_f(v);$|&\n        return k_int((long long)llround(x));|' src/runtime.c
rm -f src/runtime.c.bak
grep -q '^        return k_int((long long)llround(x));$' src/runtime.c
