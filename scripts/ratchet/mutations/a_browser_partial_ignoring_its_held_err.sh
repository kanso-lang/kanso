#!/bin/sh
# Ask none of a partial's held arguments whether it failed in the browser
# backend, as it did until 2026-09-29. The page then answers what g answers
# where native and the interpreter answer the err, and the wasm corpus goes
# red.
set -e
ask='        for &local in &locals {'
end='        for _ in &locals {'
[ "$(grep -cxF "$ask" src/wasm_backend.rs)" -eq 1 ]
[ "$(grep -cxF "$end" src/wasm_backend.rs)" -eq 1 ]
sed -i.bak -e 's#^        for &local in &locals {$#        for \&local in \&locals[..0] {#' -e 's#^        for _ in &locals {$#        for _ in \&locals[..0] {#' src/wasm_backend.rs
rm -f src/wasm_backend.rs.bak
! grep -qxF "$ask" src/wasm_backend.rs
