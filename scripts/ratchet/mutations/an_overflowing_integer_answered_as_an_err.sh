#!/bin/sh
# Answer an integer past int64 with an err value again, as the runtime did
# before 2026-09-28. A program can catch it, std/json turns it into "invalid
# number", and the spec an_integer_past_int64_reads_whole_on_every_engine
# finds the err where the whole integer should be.
set -e
grep -q '^    if (range) return k_int_of_digits(data, len);$' src/runtime.c
sed -i.bak 's|^    if (range) return k_int_of_digits(data, len);$|    if (range) return k_err(k_str("overflows this engine'"'"'s integers"), origin);|' src/runtime.c
rm -f src/runtime.c.bak
grep -q "^    if (range) return k_err(k_str(\"overflows this engine's integers\"), origin);$" src/runtime.c
