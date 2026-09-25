#!/bin/sh
# Four runtime doors are defined with K_DOORCC and called with
# preserve_nonecc. This mutation takes k_b_utf8 off the emitter's list, so the
# runtime defines it on one convention and the program calls it on the
# other; the witness is the_doors_the_program_calls_agree.
set -e
old='["k_b_append_rendered", "k_b_entries", "k_b_to_float_slice", "k_b_utf8"];'
[ "$(grep -cF "$old" src/codegen.rs)" -eq 1 ] || {
  echo "the door list moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/pub const PRESERVE_NONE_DOORS: \[&str; 4\] =/pub const PRESERVE_NONE_DOORS: [\&str; 3] =/; s/\["k_b_append_rendered", "k_b_entries", "k_b_to_float_slice", "k_b_utf8"\];/["k_b_append_rendered", "k_b_entries", "k_b_to_float_slice"];/' src/codegen.rs
grep -qF '["k_b_append_rendered", "k_b_entries", "k_b_to_float_slice"];' src/codegen.rs
