#!/bin/sh
# Leave a group's parameters as the direct calls set them when a partial hands
# the group out, as inference did until 2026-09-29. A subtype held by
# `&f1 (id 11)` then comes back out of an arm compiled for an int as its box's
# address, and the micro corpus goes red.
set -e
old='            if env.get(name).is_none() {'
[ "$(grep -cxF "$old" src/infer.rs)" -eq 1 ]
sed -i.bak 's#^            if env.get(name).is_none() {$#            if false {#' src/infer.rs
rm -f src/infer.rs.bak
! grep -qxF "$old" src/infer.rs
