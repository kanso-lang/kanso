#!/bin/sh
# Stop the checker asking whether a pattern that takes a value apart can ever
# match, as it did not until 2026-09-28 beyond a typeset, which it still
# asks. `(id n)` for `type id
# int`, `(pt n)` for a two-field `pt` and `(tag p)` for a wrapper of one then
# pass `kanso check`, and the error corpus fixture
# a_wrapper_pattern_never_matches reports nothing.
set -e
old='    let mut root = decl;'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^    let mut root = decl;$#    let mut root = decl; if taken > 0 { return None; }#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
