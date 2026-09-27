#!/bin/sh
# `values` reads the same sorted view `keys` and `entries` do, which is what
# lines position i of each up with the others. This reads the values in the
# order they were put instead, so a map whose keys arrived out of order hands
# back its values against the wrong keys, and the alignment golden says false.
set -e
t='    for (long long i = 0; i < n; i++) items[i] = s[i * 2 + side];'
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the column read changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/    for (long long i = 0; i < n; i++) items\[i\] = s\[i \* 2 + side\];/    KValue* from = side ? m->pairs : s;\n    for (long long i = 0; i < n; i++) items[i] = from[i * 2 + side];/' src/runtime.c
grep -qF 'KValue* from = side ? m->pairs : s;' src/runtime.c
