#!/bin/sh
# Let a subtype wrap a typeset, as `kanso check` did until 2026-09-29. The
# interpreter then builds `blob (circle 1)` and prints it, the native backend
# refuses it at run time, and the error corpus goes red.
set -e
old='        if let Some(parent) = ty.parent.as_deref().filter(|p| typeset(p)) {'
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        if let Some(parent) = ty.parent.as_deref().filter(|p| typeset(p)) {$#        if let Some(parent) = ty.parent.as_deref().filter(|p| typeset(p) \&\& false) {#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
