#!/bin/sh
# Leave the walk's diagnostics without the file of the declaration they are
# about, as they were until 2026-09-29. In a module a build block's refusal then
# names the module and no location, and the error corpus goes red on every
# fixture whose imported golden carries one.
set -e
old='                d.file = Some(std::sync::Arc::clone(&decl.file));'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^                d.file = Some(std::sync::Arc::clone(&decl.file));$#                let _ = \&decl.file;#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
