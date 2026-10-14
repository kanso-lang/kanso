#!/bin/sh
# A pooled cell reaches the pool only when the sleep carrying it parks, so a
# chain that is never run hands its work to nobody. This mutation queues the
# cells when the sleep is built, and the mem golden of a chain never run reads
# `ahead_forced=1` where it read 0.
set -e
line='    k_as_desc(d)->y = k_mklist(n < 8 ? n : 8, held);'
[ "$(grep -cxF "$line" src/runtime/effects.c)" -eq 1 ] || {
  echo "the pooled sleep's descriptor moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    k_as_desc(d)->y = k_mklist(n < 8 ? n : 8, held);$/    k_ahead_enqueue(k_mklist(n < 8 ? n : 8, held));/' src/runtime/effects.c
if grep -qxF "$line" src/runtime/effects.c; then exit 1; fi
