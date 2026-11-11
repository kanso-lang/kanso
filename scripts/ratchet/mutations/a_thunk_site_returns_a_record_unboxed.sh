#!/bin/sh
# Forget that a tail call into a register-returned record came back as two
# words, as the emitter did before 2026-09-28.
#
# A lazy cell's site returns the pair as a tagged value without boxing it, and
# clang refuses the micro fixture a_lazy_record_call_somebody_makes_still_builds.
#
# The call that records it runs over six lines since the record's field order
# became an argument (kanso#2431), so the patch takes the lines from its
# opening to its close, both at the site's own indentation.
set -e
open='^ \{32\}f\.record_parsed($'
shut='^ \{32\});$'
test "$(grep -c "$open" src/codegen/emit.rs)" = 1
sed -i.bak "/$open/,/$shut/c\\
                                let _ = ty;
" src/codegen/emit.rs
rm -f src/codegen/emit.rs.bak
if grep -q "$open" src/codegen/emit.rs; then exit 1; fi
grep -q '^ \{32\}let _ = ty;$' src/codegen/emit.rs
