#!/bin/sh
# Turn a group value's failing first argument down before entering the group,
# naming the group as its own guard would. The trace is the same as the
# group's, but an `(err _)` arm never sees the err, and the micro corpus
# fixture answers the err where the interpreter answers "caught".
set -e
old='        return ((KValue(*)(KValue, KValue))r->fn)(a, b);'
[ "$(grep -cxF "$old" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^        return ((KValue(\*)(KValue, KValue))r->fn)(a, b);$#        if (!k_not_failure(a)) return k_err_hop(a, r->name);\n        return ((KValue(*)(KValue, KValue))r->fn)(a, b);#' src/runtime.c
rm -f src/runtime.c.bak
grep -qxF '        if (!k_not_failure(a)) return k_err_hop(a, r->name);' src/runtime.c
