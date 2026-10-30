#!/bin/sh
# Leave a carried record's int tag out of the first word, in the emitter's
# pack and in the runtime's unpack of a boxed record. A bignum position then
# crosses as a word holding its pointer, and the micro corpus goes red on
# an_int_of_any_width_crosses_in_its_own_word.
set -e
old='    f.line_fmt(format_args!("{high} = shl i64 {ptag}, 8"));'
[ "$(grep -cxF "$old" src/codegen/words.rs)" -eq 1 ]
sed -i.bak 's#^    f.line_fmt(format_args!("{high} = shl i64 {ptag}, 8"));$#    f.line_fmt(format_args!("{high} = and i64 {ptag}, 0"));#' src/codegen/words.rs
rm -f src/codegen/words.rs.bak
! grep -qxF "$old" src/codegen/words.rs
words='    out.tag = (long long)((unsigned long long)f0.tag << 8) | f1.tag;'
[ "$(grep -cxF "$words" src/runtime/values.c)" -eq 1 ]
sed -i.bak 's#^    out.tag = (long long)((unsigned long long)f0.tag << 8) | f1.tag;$#    out.tag = f1.tag;#' src/runtime/values.c
rm -f src/runtime/values.c.bak
! grep -qxF "$words" src/runtime/values.c
