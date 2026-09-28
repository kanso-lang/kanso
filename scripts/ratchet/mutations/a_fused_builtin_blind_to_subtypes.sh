#!/bin/sh
# Stop the builtins the compiler writes out itself reading through a subtype,
# as they did not until 2026-09-28: `bytes` bound to a name, and `utf8`,
# `append`, `to_int` and `to_float` over a `slice`. A slice then refuses a
# subtype of int as a position, and the micro fixture
# a_fused_builtin_reads_through_a_subtype dies where the interpreter prints.
set -e
old='        if self.sub_parents.is_empty() {'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        if self.sub_parents.is_empty() {$#        if true {#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
