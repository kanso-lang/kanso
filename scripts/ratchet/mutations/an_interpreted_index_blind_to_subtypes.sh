#!/bin/sh
# Stop the interpreter's index reading through a subtype, as it did not until
# 2026-09-28. It then refuses a subtype of int as a position and as a map
# key, and the micro fixture a_subtype_indexes_as_its_base stops where a
# compiled build prints.
set -e
old='                    Value::Sub { .. } => sub_base(v),'
[ "$(grep -cF "$old" src/eval.rs)" -ge 1 ]
n=$(grep -nF 'let base_of = |v: Value| match v {' src/eval.rs | cut -d: -f1)
[ -n "$n" ]
sed -i.bak "$((n + 1))s#^                    Value::Sub { .. } => sub_base(v),\$#                    Value::Sub { .. } if false => sub_base(v),#" src/eval.rs
rm -f src/eval.rs.bak
sed -n "$((n + 1))p" src/eval.rs | grep -qF 'if false'
