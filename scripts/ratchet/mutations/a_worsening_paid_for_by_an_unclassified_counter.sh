# A counter WITH a direction worsens beside one that has none. The gate must
# refuse the first and not let the second's silence excuse it.
#
# The pure-regression rule reads "something got worse and nothing got better".
# A counter no direction table names cannot answer either half, so it has to
# count toward neither side; the third state exists so the listing says which
# moves were classified and which were only observed.
#
# THE UNCLASSIFIED HALF IS `thunk_forces` AND THE CHOICE IS LOAD-BEARING. It
# was `evac_allocs` until 2026-09-01, when the classification sweep gave that
# counter a direction — at which point this row went on turning the gate red
# while testing nothing it was written to test, because BOTH halves were
# classified worsenings and any pure-regression rule refuses those. The
# counter here has to be one that stays unclassified on purpose, and
# `thunk_forces` is: it counts asking a thunk for its value rather than
# computing one, so a change that made a program strict would lower it while
# doing identical work. tests/a_mutation_keeps_its_unclassified_counter.rs
# holds the two ends of that together.
#
# `scanbench calls` is the classified half. Both are bare numbers rather than
# anchors on today's figures: the goldens' real rows move whenever the emitter
# or the runtime does, and an anchor naming today's value would stop applying
# the next time one moves.
set -e
sed -i 's/^scanbench defines=\([0-9]*\) calls=[0-9]*/scanbench defines=\1 calls=999999/' \
  bench/emitted_golden_others.txt
sed -i 's/^thunk_forces=[0-9]*/thunk_forces=999999/' bench/cost_golden_digest.txt
grep -q '^scanbench .*calls=999999' bench/emitted_golden_others.txt
grep -q '^thunk_forces=999999' bench/cost_golden_digest.txt
