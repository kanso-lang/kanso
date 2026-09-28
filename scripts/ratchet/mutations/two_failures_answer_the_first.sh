#!/bin/sh
# Answer a builtin's first failing argument alone, as the C did before
# 2026-09-28, where the interpreter merges every failing argument.
set -e
sed -i.bak 's/^        out = have ? k_accumulate_failures(out, xs\[i\]) : xs\[i\];$/        if (!have) out = xs[i];/' src/runtime.c
rm -f src/runtime.c.bak
grep -q '^        if (!have) out = xs\[i\];$' src/runtime.c
