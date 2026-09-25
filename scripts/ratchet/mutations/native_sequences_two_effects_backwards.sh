#!/bin/sh
# `>>` runs its right side first on native. `io/write "a" >> io/write "b"`
# prints `ba` where the oracle prints `ab`.
#
# The executor is written twice — the interpreter's and the C runtime's — with
# no shared code between them, and only the effects sweep asks what happens
# when two effects are SEQUENCED rather than what one of them returns. The wall
# is ordered, which is the whole content of the rule this breaks.
set -e
target='return k_mkdesc(1, a, b);'
grep -qF "$target" src/runtime.c || {
  echo "the sequencing primitive moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|return k_mkdesc(1, a, b);|return k_mkdesc(1, b, a);|' src/runtime.c
grep -qF 'return k_mkdesc(1, b, a);' src/runtime.c
