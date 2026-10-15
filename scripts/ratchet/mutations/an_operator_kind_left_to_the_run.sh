#!/bin/sh
# An arithmetic operator handed a literal string, list or map is refused at
# check (#1975). This mutation stops asking, so the error corpus holds a
# diagnostic nobody raised and the program reaches the run instead.
set -e
line='                wrong_kind_at_an_operator(cur, returns, &shadow, none_diags);'
[ "$(grep -cxF "$line" src/check/after_infer.rs)" -eq 1 ] || {
  echo "the operator kind check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                wrong_kind_at_an_operator(cur, returns, &shadow, none_diags);$/                let _ = wrong_kind_at_an_operator;/' src/check/after_infer.rs
if grep -qxF "$line" src/check/after_infer.rs; then exit 1; fi
