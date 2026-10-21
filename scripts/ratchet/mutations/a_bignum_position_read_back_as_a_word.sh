#!/bin/sh
# Read a carried record's int back as a word whatever its tag says, in the
# pattern that takes one apart and in the runtime that boxes one. A bignum
# position then comes back as its pointer printed as an int, and the micro
# corpus goes red on an_int_of_any_width_crosses_in_its_own_word.
set -e
old='    f.line_fmt(format_args!("{ptag} = lshr i64 {w0}, 8"));'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^    f.line_fmt(format_args!("{ptag} = lshr i64 {w0}, 8"));$#    f.line_fmt(format_args!("{ptag} = and i64 {w0}, 0"));#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
box='    fields[0].tag = (unsigned long long)w0 >> 8; fields[0].payload = w2;'
[ "$(grep -cxF "$box" src/runtime/values.c)" -eq 1 ]
sed -i.bak 's#^    fields\[0\].tag = (unsigned long long)w0 >> 8; fields\[0\].payload = w2;$#    fields[0].tag = K_INT; fields[0].payload = w2;#' src/runtime/values.c
rm -f src/runtime/values.c.bak
! grep -qxF "$box" src/runtime/values.c
