#!/bin/sh
# Let the interpreter's `ref` answer whichever tie is open, as naming its tie
# by position on the stack did before 2026-10-05. A `ref` called after its tie
# returned, from inside a second tie's maker, then answers the second tie's
# cell instead of being refused, and the runtime corpus goes red.
set -e
old='                let Some(tie) = ties.iter_mut().rev().find(|t| t.serial == serial) else {'
[ "$(grep -cxF "$old" src/eval.rs)" -eq 1 ]
sed -i.bak 's#^                let Some(tie) = ties.iter_mut().rev().find(|t| t.serial == serial) else {$#                let Some(tie) = ties.iter_mut().rev().find(|t| t.serial == serial || serial > 0) else {#' src/eval.rs
rm -f src/eval.rs.bak
! grep -qxF "$old" src/eval.rs
