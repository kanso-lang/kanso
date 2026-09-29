#!/bin/sh
# Stop the checker asking whether an earlier typeset arm holds every member of
# a later one, as it did not until 2026-09-28. A group with `_:wide` above
# `_:narrow` then passes `kanso check`, and the error corpus fixture
# a_typeset_arm_behind_a_wider_one reports nothing.
set -e
old='        if !n.iter().all(|m| w.contains(m)) {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        if !n.iter().all(|m| w.contains(m)) {$#        if true {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
