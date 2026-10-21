#!/bin/sh
# Keep a group's read links from its first call, whatever the others pass.
# `step` in a_position_read_beside_its_byte_stays_exact is called once with
# a read beside its position and once with a literal byte beside the largest
# word; linked anyway, its `p + 1` loses the overflow test in a release build
# and wraps, and the release-built micro corpus goes red. The dev tier never
# acts on the assume, so the corpus it builds stays green.
set -e
old='                    .and_modify(|links| links.retain(|l| here.contains(l)))'
[ "$(grep -cxF "$old" src/read_at.rs)" -eq 1 ] || {
  echo "the call sites' intersection moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    \.and_modify(|links| links\.retain(|l| here\.contains(l)))$/                    .and_modify(|_| ())/' src/read_at.rs
! grep -qxF "$old" src/read_at.rs
