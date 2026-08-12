#!/bin/sh
# Native words a refusal the oracle words differently. The map-key message is
# written once in src/eval.rs and once in src/runtime.c, and only the second is
# touched here, so the two engines answer the same program with two sentences.
set -e
sed -i.bak 's/%.\*s is not usable as a map key/%.*s cannot be a map key/' src/runtime.c
rm -f src/runtime.c.bak
grep -q 'cannot be a map key' src/runtime.c
