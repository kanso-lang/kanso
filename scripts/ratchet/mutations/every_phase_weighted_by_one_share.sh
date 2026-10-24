#!/bin/sh
# welfare_rescore weighs each phase by its own share, read by the phase's name
# from the run program's header table (2026-10-24). This mutation gives every
# phase the decode phase's share, so a history whose phases moved apart is
# rebuilt on equal weights: the uneven fixture in
# tests/the_reconstruction_refuses_a_splice_that_moved.rs reads 919 where its
# shares give 762.
set -e
target='  [parts[2] (share_of shares parts[1])]'
n=$(grep -cxF "$target" scripts/welfare_rescore/welfare_rescore.kso)
[ "$n" -eq 1 ] || { echo "the share lookup changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^  \[parts\[2\] (share_of shares parts\[1\])\]$/  [parts[2] (share_of shares "decode")]/' scripts/welfare_rescore/welfare_rescore.kso
grep -qxF '  [parts[2] (share_of shares "decode")]' scripts/welfare_rescore/welfare_rescore.kso
