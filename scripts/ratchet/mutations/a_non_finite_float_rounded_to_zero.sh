#!/bin/sh
# Round NaN and the infinities to 0 in the interpreter, where every engine
# answers `none` since 2026-09-29. The oracle then prints 0 for all three, and
# the micro corpus goes red.
set -e
old='                    Value::Float(_) => Ok(Value::NoneV),'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^                    Value::Float(_) => Ok(Value::NoneV),$#                    Value::Float(_) => Ok(Value::int(0i64)),#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
