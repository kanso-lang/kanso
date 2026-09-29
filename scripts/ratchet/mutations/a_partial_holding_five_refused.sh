#!/bin/sh
# Refuse a native partial that holds more than four, as the emitter did until
# 2026-09-29. `&f6 1 2 3 4 5` then fails to build where the interpreter runs
# it, and the micro corpus goes red.
set -e
old='        let mut rest: &[String] = &held;'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        let mut rest: &\[String\] = &held;$#        if held.len() > 4 {\n            return Err("native backend: a partial holds at most 4 arguments".to_string());\n        }\n        let mut rest: \&[String] = \&held;#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
grep -qF 'a partial holds at most 4 arguments' src/codegen.rs
