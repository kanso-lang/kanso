#!/bin/sh
# The length twin asks "list or bytes" as one compare on tag|4. Written as
# two compares on the tag, LLVM folds them to that itself -- until the string
# arm adds a third compare on the same value, when SimplifyCFG gathers all
# three into a switch and lowers the switch as a chain: six instructions where
# the list-length test in encodebench's pair loop had two, 3,344,400 times a
# run. A compare on a different value is not a switch case, so the fold is
# written out. This mutation writes the two compares back, and the work vein
# is the witness: encodebench rose 15,586,000 on that row when the chain
# first landed, and it rises again when this runs.
set -e
grep -q '^  %t4 = or i64 %tag, 4$' src/codegen.rs || {
  echo "the length twin's tag|4 fold changed shape; this needs rewriting" >&2
  exit 1
}
grep -q '^  %fastable = icmp eq i64 %t4, 13$' src/codegen.rs
sed -i 's|^  %t4 = or i64 %tag, 4$|  %is_list = icmp eq i64 %tag, 9\n  %is_bytes = icmp eq i64 %tag, 13|' \
  src/codegen.rs
sed -i 's|^  %fastable = icmp eq i64 %t4, 13$|  %fastable = or i1 %is_list, %is_bytes|' \
  src/codegen.rs
grep -q '^  %fastable = or i1 %is_list, %is_bytes$' src/codegen.rs
