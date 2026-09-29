#!/bin/sh
# Refuse a group of more than eight parameters as a value, as native did until
# 2026-09-29. `g = f9` then fails to build where the interpreter runs it, a
# partial over f10 holding nine goes with it, and the micro corpus goes red.
set -e
old='                if arities.len() == 1 && arities[0] >= 1 && self.simple_fn_value(name, arities[0]) {'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#\&\& arities\[0\] >= 1 \&\& self.simple_fn_value#\&\& (1..=8).contains(\&arities[0]) \&\& self.simple_fn_value#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
grep -qF '(1..=8).contains(&arities[0]) && self.simple_fn_value' src/codegen.rs
