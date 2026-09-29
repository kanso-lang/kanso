#!/bin/sh
# Leave a type declaration's refusal without the declaration's file, as the
# merged check of a module did until 2026-09-29. Through an import, a subtype
# of `none` or of a typeset then names the module and no place, and the error
# corpus goes red.
set -e
old='        if !ty.file.is_empty() {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        if !ty.file.is_empty() {$#        if false {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
