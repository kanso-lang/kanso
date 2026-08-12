#!/bin/sh
# Native says something the oracle does not, which is the divergence the
# differential exists to refuse.
set -e
sed -i.bak 's/a lazy binding demands its own value/a lazy binding wants itself/' src/runtime.c
rm -f src/runtime.c.bak
grep -q 'a lazy binding wants itself' src/runtime.c
