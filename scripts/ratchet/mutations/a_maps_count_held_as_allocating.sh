#!/bin/sh
# `entry_count` is pure to the beat analysis, as `key_at` and `value_at` are,
# so the encoder's descent into a map is no region. This mutation lists it as
# allocating again: each map gets a region, and inside one the builder sits
# below the mark and grows in malloc'd storage. The output is the same; the
# witness is the run program's counters.
set -e
line='        "entry_count",'
[ "$(grep -cxF "$line" src/beat.rs)" = 1 ] || {
  echo "entry_count's place in the beat lists moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^        "entries",$/a\        "entry_count",' src/beat.rs
[ "$(grep -cxF "$line" src/beat.rs)" = 2 ]
