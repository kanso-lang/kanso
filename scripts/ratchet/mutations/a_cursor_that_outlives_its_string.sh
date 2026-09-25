#!/bin/sh
# The seek cursor, kept across a beat's rewind.
#
# Reading a character out of a string by position remembers where the walk
# stopped so the next read resumes rather than crossing the text again. The
# remembered position names its string by address. `k_beat_rewind` moves the
# arena back, every address above the mark becomes free to hand out again, and
# the next string the loop builds can land exactly where the last one sat — so
# a cursor kept from the previous iteration is a cursor into a string that is
# no longer there.
#
# That is kanso#1173. Native answered `g` where character five was `e`, and on
# some layout pairs it answered a byte from the middle of a codepoint. The
# store below is the whole fix; deleting it restores the defect.
#
# Under the mutation the beat sweep reports 16 of its 96 layout pairs
# disagreeing. It is caught by the micro corpus too — one golden pins one
# shape — which is what a mutation should be: proof this gate runs and
# reddens, not a claim that only it can see the defect.
#
# There are two resets now, not one. The rewind's empty test moved inline to
# its call sites and took the three stores with it, so the reset appears once
# in `k_beat_rewind_slow` and once in the inline fast path — at a different
# indentation, which is how this mutation caught the move and said so rather
# than quietly deleting half of it. Both have to go: leaving either standing
# means the path that runs still resets the cursor and the defect never
# appears.
set -e
before=$(grep -cE '^ +k_seek_str = NULL;$' src/runtime.c)
if [ "$before" -lt 2 ]; then
  echo "expected two cursor resets in the rewind, found $before;" >&2
  echo "the reset moved again and this mutation needs rewriting" >&2
  exit 1
fi
sed -i '/^ *k_seek_str = NULL;$/d' src/runtime.c
if grep -qE '^ +k_seek_str = NULL;$' src/runtime.c; then
  echo "a cursor reset is still there; the mutation did nothing" >&2
  exit 1
fi
