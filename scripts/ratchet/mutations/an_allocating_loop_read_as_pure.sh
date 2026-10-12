#!/bin/sh
# A loop that allocates and carries nothing across its rewind is read as a
# pure loop, and gets no beat.
#
# readbench's `tally` is that loop: it splits the document every round and
# hands the document on unchanged, so nothing crosses the rewind. Without the
# beat its arena is never rewound, and beat_iters reads 1 where the golden
# has 201.
set -e
f=src/beat.rs
grep -q "fn classify" src/beat.rs
line='    Some(crate::beat::Verdict::Beat)'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the plain beat verdict moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    Some(crate::beat::Verdict::Beat)$/    Some(crate::beat::Verdict::PureLoop)/' "$f"
grep -qxF '    Some(crate::beat::Verdict::PureLoop)' "$f"
