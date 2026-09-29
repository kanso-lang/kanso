#!/bin/sh
# Leave partials off the values equality refuses, as the interpreter did until
# 2026-09-29. `(&f n) == 3` then answers false where native refuses, and the
# runtime corpus goes red.
set -e
old='        Value::FnRef(_) | Value::Closure(_) | Value::Desc(_) | Value::Partial(..) => true,'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^        Value::FnRef(_) | Value::Closure(_) | Value::Desc(_) | Value::Partial(..) => true,$#        Value::FnRef(_) | Value::Closure(_) | Value::Desc(_) => true,#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
