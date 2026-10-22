#!/bin/sh
# Scan a shared token even when it is known to be clean.
#
# The answer is the same either way, so no output moves: what moves is
# `find2_calls` in the encode vein. encodebench carries its own copy of
# lib/json, whose encoder still sends every string through `escape_onto`; the
# library's own encoder no longer does, so the run program cannot see this.
set -e
sed -i.bak 's/^        if (off < sizeof(k_token_store) \&\& k_token_clean\[off \/ sizeof(KToken)\]) return len + 1;$/        if (0 \&\& off < sizeof(k_token_store) \&\& k_token_clean[off \/ sizeof(KToken)]) return len + 1;/' src/runtime/text.c
rm -f src/runtime/text.c.bak
grep -q '^        if (0 && off < sizeof(k_token_store)' src/runtime/text.c
