#!/bin/sh
# Answer no count for a partial in the browser runtime, as it did until
# 2026-09-29. `s = &k` over `k = &h 3` then grows at `s 1` and the page prints
# `<fn>` where the interpreter runs g, and the wasm corpus goes red.
set -e
old='        return arities_of(held[0]).into_iter().filter(|a| *a >= n).map(|a| a - n).collect();'
[ "$(grep -cxF "$old" src/wasm_rt.rs)" -eq 1 ]
sed -i.bak 's#^        return arities_of(held\[0\]).into_iter().filter(|a| \*a >= n).map(|a| a - n).collect();$#        return Vec::new();#' src/wasm_rt.rs
rm -f src/wasm_rt.rs.bak
! grep -qxF "$old" src/wasm_rt.rs
