#!/bin/sh
# Read a packed record's int back with a logical shift, as the native backend
# did until 2026-09-29, in the pattern that takes one apart and in the runtime
# that boxes one. A negative int then comes back as 2^56 plus it, and the
# micro corpus and the packed-record entry spec go red.
set -e
old='    f.line(&format!("{n} = ashr i64 {w0}, 8"));'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^    f.line(&format!("{n} = ashr i64 {w0}, 8"));$#    f.line(\&format!("{n} = lshr i64 {w0}, 8"));#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
box='    fields[0].tag = K_INT; fields[0].payload = w0 >> 8;'
[ "$(grep -cxF "$box" src/runtime.c)" -eq 1 ]
sed -i.bak 's#^    fields\[0\].tag = K_INT; fields\[0\].payload = w0 >> 8;$#    fields[0].tag = K_INT; fields[0].payload = (long long)((unsigned long long)w0 >> 8);#' src/runtime.c
rm -f src/runtime.c.bak
! grep -qxF "$box" src/runtime.c
