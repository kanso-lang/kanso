#!/bin/sh
# An arena builder at the arena's frontier grows by moving the frontier past
# it. This mutation never takes that path, so every grow in the arena takes a
# new buffer and copies. The output is the same; the witness is the mem
# corpus, where three fixtures grow builders in the arena.
set -e
line='    if (mutate && dies && (a->cap & 1) && (a->cap & ~1LL)'
grep -qxF "$line" src/runtime/text.c || {
  echo "the in-place grow's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (mutate \&\& dies \&\& (a->cap \& 1) \&\& (a->cap \& ~1LL)$/    if (0 \&\& mutate \&\& dies \&\& (a->cap \& 1) \&\& (a->cap \& ~1LL)/' src/runtime/text.c
if grep -qxF "$line" src/runtime/text.c; then exit 1; fi
