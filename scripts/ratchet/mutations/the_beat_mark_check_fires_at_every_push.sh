#!/bin/sh
# The beat mark's sanity check compares the mark's own pointer and remaining
# count against the end of its own block, and it moved from the rewind to the
# push: a mark is written once and never again, so the rewind was asking a
# question already answered, at eight instructions an iteration.
#
# The move is only sound if the check still RUNS. Nothing else in the corpus
# would notice its removal — it is a diagnostic, and a diagnostic that never
# fires looks exactly like one that was deleted. So the mutation inverts it:
# the comparison that should hold at every push now fails at every push, and
# every program with a beat loop dies at the first one. A corpus that stays
# green under this is a corpus in which the check is gone.
set -e
grep -qF "if (k_blocks && m->ptr + m->left != (char*)(k_blocks + 1) + k_blocks->cap) {" src/runtime.c || {
  echo "the beat mark check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|if (k_blocks \&\& m->ptr + m->left != (char\*)(k_blocks + 1) + k_blocks->cap) {|if (k_blocks \&\& m->ptr + m->left == (char*)(k_blocks + 1) + k_blocks->cap) {|' \
  src/runtime.c
grep -qF "if (k_blocks && m->ptr + m->left == (char*)(k_blocks + 1) + k_blocks->cap) {" src/runtime.c
