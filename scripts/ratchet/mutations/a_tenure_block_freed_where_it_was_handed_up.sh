#!/bin/sh
# A beat whose result is a heap value hands its depth's tenure blocks to the
# depth outside; one that rewound frees them where they are. This mutation
# frees them in both cases.
#
# Until `ten_handups` existed the vein could not see the difference. The block
# is allocated once and freed once either way -- the hand-up only moves WHERE
# the free happens -- so `ten_blocks` and `ten_frees` read identically, and
# replacing the call left all sixty-seven .mem goldens byte-identical. The
# fixture a_repaired_node_below_the_mark_holds_tenure carried a comment saying
# a change of exactly this shape would turn it red. It did not.
#
# With the counter, three fixtures go red on the swap:
# a_carried_value_written_into_an_older_node and
# a_repaired_node_below_the_mark_holds_tenure each read ten_handups=1 against a
# golden of 0, and an_inner_beat_opens_its_tenure_in_the_block_outside reads 4
# against 0 -- the last one also moving ten_blocks 5 -> 3, which is the only
# part of this any counter could see before.
set -e
grep -q '^    else k_ten_hand_up(d);$' src/runtime.c || {
  echo "the beat pop's hand-up call changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    else k_ten_hand_up(d);$|    else k_ten_release(d);|' src/runtime.c
grep -q '^    else k_ten_release(d);$' src/runtime.c
