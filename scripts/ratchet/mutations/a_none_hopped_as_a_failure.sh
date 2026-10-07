#!/bin/sh
# Hop a none out of a dispatcher that matched no arm, as a compiled build did
# until 2026-09-28. A none is a value (ruled 2026-07-24), and the runtime
# corpus fixture a_none_with_no_arm_is_refused then reads nothing where the
# refusal belongs.
set -e
old='            f.line_fmt(format_args!("{failing} = icmp eq i64 {disc_fail}, 5"));'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^            f.line_fmt(format_args!("{failing} = icmp eq i64 {disc_fail}, 5"));$#            f.line_fmt(format_args!("{failing}.e = icmp eq i64 {disc_fail}, 5"));\
            f.line_fmt(format_args!("{failing}.n = icmp eq i64 {disc_fail}, 4"));\
            f.line_fmt(format_args!("{failing} = or i1 {failing}.e, {failing}.n"));#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
grep -qF '{failing} = or i1 {failing}.e, {failing}.n' src/codegen.rs
