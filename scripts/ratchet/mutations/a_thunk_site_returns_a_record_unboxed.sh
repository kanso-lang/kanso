#!/bin/sh
# Forget that a tail call into a register-returned record came back as two
# words, as the emitter did before 2026-09-28.
#
# A lazy cell's site returns the pair as a tagged value without boxing it, and
# clang refuses the micro fixture a_lazy_record_call_nobody_reads_still_builds.
set -e
grep -q '^                                f.record_parsed(&t, ty, self.type_ids\[ty\]);$' src/codegen.rs
sed -i.bak 's|^                                f.record_parsed(&t, ty, self.type_ids\[ty\]);$|                                let _ = ty;|' src/codegen.rs
rm -f src/codegen.rs.bak
if grep -q '^                                f.record_parsed(&t, ty, self.type_ids\[ty\]);$' src/codegen.rs; then exit 1; fi
