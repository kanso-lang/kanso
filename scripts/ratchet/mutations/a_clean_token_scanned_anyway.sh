#!/bin/sh
# Scan a shared token even when it is known to be clean.
#
# The answer is the same either way, so no output moves: what moves is the
# work the run program does, which the work vein pins.
set -e
sed -i.bak 's/^        if (off < sizeof(k_token_store) \&\& k_token_clean\[off \/ sizeof(KToken)\]) return len + 1;$/        if (0 \&\& off < sizeof(k_token_store) \&\& k_token_clean[off \/ sizeof(KToken)]) return len + 1;/' src/runtime.c
rm -f src/runtime.c.bak
grep -q '^        if (0 && off < sizeof(k_token_store)' src/runtime.c
