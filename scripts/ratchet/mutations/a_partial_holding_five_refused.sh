#!/bin/sh
# Refuse a native partial that holds more than four, as the emitter did until
# 2026-09-29. `&f6 1 2 3 4 5` then fails to build where the interpreter runs
# it, and the micro corpus goes red.
set -e
old='        if n > 8 {'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        if n > 8 {$#        if n > 4 {#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
