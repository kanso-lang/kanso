#!/bin/sh
# Answer an integer past int64 with an err value again, as the runtime did
# before 2026-09-28. A program can catch it, std/json turns it into "invalid
# number", and the spec an_integer_past_int64_is_refused_natively finds no
# refusal in the compiled build's output.
set -e
grep -q '^    if (range)$' src/runtime.c
sed -i.bak '/^    if (range)$/{n;s|^        k_die(.*|        return k_err(k_str("overflows this engine'"'"'s integers"), origin);|;}' src/runtime.c
rm -f src/runtime.c.bak
grep -q "^        return k_err(k_str(\"overflows this engine's integers\"), origin);$" src/runtime.c
