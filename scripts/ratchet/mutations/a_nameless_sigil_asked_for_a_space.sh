#!/bin/sh
# Hold a sigil with no name after it to the operator's spacing, as the lexer
# did until 2026-09-29. `k = &(g 1)` then asks for a space where `& (g 1)`
# fails to parse, and the error corpus goes red.
set -e
old='        if amp && !value_before && !matches!(next, Tok::Ident(_)) {'
[ "$(grep -cxF "$old" src/lexer.rs)" -eq 1 ]
sed -i.bak 's#^        if amp && !value_before && !matches!(next, Tok::Ident(_)) {$#        if false {#' src/lexer.rs
rm -f src/lexer.rs.bak
! grep -qxF "$old" src/lexer.rs
