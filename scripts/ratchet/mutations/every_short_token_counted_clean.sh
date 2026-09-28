#!/bin/sh
# Mark every shared token clean, whatever its bytes.
#
# The flag is decided when a token's slot fills. If it is wrong, the JSON
# writer copies a quote, a backslash or a control character straight into
# its output, and a_short_token_with_a_quote_is_still_escaped reads that.
set -e
sed -i.bak 's/^        k_token_clean\[slot\] = clean;$/        k_token_clean[slot] = 1;/' src/runtime.c
rm -f src/runtime.c.bak
grep -q '^        k_token_clean\[slot\] = 1;$' src/runtime.c
