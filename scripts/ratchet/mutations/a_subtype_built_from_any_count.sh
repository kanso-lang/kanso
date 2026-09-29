#!/bin/sh
# Let a subtype be built from any number of values, as `kanso check` did until
# 2026-09-29. `tall 1 2` then passes the check, the interpreter refuses it at
# run time and the native backend refuses the build, and the error corpus goes
# red.
set -e
old='        Some(parent) if argc != 1 => Some(format!('
[ "$(grep -cxF "$old" src/check.rs)" -eq 1 ]
sed -i.bak 's#^        Some(parent) if argc != 1 => Some(format!($#        Some(parent) if argc == usize::MAX => Some(format!(#' src/check.rs
rm -f src/check.rs.bak
! grep -qxF "$old" src/check.rs
