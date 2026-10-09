#!/bin/sh
# A push the analysis proved in place writes through the pointer and hands back
# the `Rc` it was given. This mutation boxes the grown list afresh, as the
# interpreter did until 2026-10-09: the same list, one more allocation a lap,
# and the shared-seed spec's per-lap count is the witness.
set -e
kept='                    return Ok(Value::List(items));'
[ "$(grep -cxF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the in-place push moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                    return Ok(Value::List(items));$/                    return Ok(Value::List(Rc::new(Self::taken_in_place(\&items))));/' src/eval.rs
if grep -qxF "$kept" src/eval.rs; then exit 1; fi
