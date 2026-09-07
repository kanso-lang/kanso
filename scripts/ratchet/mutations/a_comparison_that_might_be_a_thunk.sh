#!/bin/sh
# The set recorded on emit_binop_builtin's merge phi goes back to TOP for the
# six comparisons, which is what an unrecorded phi read as before 2026-09-07:
# `set_of` answers `unwrap_or(TOP)` and TOP contains THUNK, so `maybe_force`
# emits `k_force_fast` in front of every `if` over a comparison the inference
# could not prove pure-int. The programs answer the same bytes either way --
# forcing a value that is not a thunk returns it -- so the checksum gate stays
# green; what moves is the emitted code, because those calls come back. This
# row is what keeps the set.
#
# TOP contains ERR as well, so this row also takes out the fold the sibling
# row below it watches. That is why the sibling exists: it puts ERR back and
# nothing else, so a set that stops proving the value is a boolean is caught
# separately from a set that stops proving it is not a thunk.
set -e
old='                _ => (f.set_of(a) \& FAIL) | (f.set_of(b) \& FAIL) | infer::BOOL,'
grep -qF '                _ => (f.set_of(a) & FAIL) | (f.set_of(b) & FAIL) | infer::BOOL,' \
  src/codegen.rs || {
  echo "the comparison phi's recorded set changed shape; this needs rewriting" >&2
  exit 1
}
sed -i "s@^$old\$@                _ => crate::infer::TOP,@" src/codegen.rs
grep -qF '                _ => crate::infer::TOP,' src/codegen.rs
