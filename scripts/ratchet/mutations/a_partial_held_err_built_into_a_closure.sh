#!/bin/sh
# Ask none of a partial's held arguments whether it failed, as native did
# until 2026-09-29. `&g p v` with `v` an err is then a function, g answers
# through it, and the micro corpus goes red.
set -e
old="        let asked: Vec<String> = held.into_iter().rev().filter(|t| !t.starts_with('{')).collect();"
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak "s#^        let asked: Vec<String> = held.into_iter().rev().filter(|t| !t.starts_with('{')).collect();\$#        let asked: Vec<String> = held.into_iter().rev().filter(|_| false).collect();#" src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
