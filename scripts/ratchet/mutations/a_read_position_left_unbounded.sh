#!/bin/sh
# Find no read links at all, so no arm learns that its position is inside
# what it indexes. The decoder's additions to a position take their overflow
# tests back, and the emitted goldens read them.
set -e
old='    found.retain(|(name, _), links| !refused.contains(name) && !links.is_empty());'
[ "$(grep -cxF "$old" src/read_at.rs)" -eq 1 ] || {
  echo "the read links' last filter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    found\.retain(|(name, _), links| !refused\.contains(name) \&\& !links\.is_empty());$/    found.clear();/' src/read_at.rs
! grep -qxF "$old" src/read_at.rs
