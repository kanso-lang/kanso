#!/bin/sh
# Six runtime doors are defined with K_DOORCC and called with
# preserve_nonecc. This mutation takes k_b_utf8 off the emitter's list, so the
# runtime defines it on one convention and the program calls it on the
# other; the witness is the_doors_the_program_calls_agree.
set -e
size='pub const PRESERVE_NONE_DOORS: [&str; 6] = ['
door='    "k_b_utf8",'
for l in "$size" "$door"; do
  [ "$(grep -cxF "$l" src/codegen.rs)" -eq 1 ] || {
    echo "the door list moved; this mutation needs rewriting" >&2
    exit 1
  }
done
sed -i 's/^pub const PRESERVE_NONE_DOORS: \[&str; 6\] = \[$/pub const PRESERVE_NONE_DOORS: [\&str; 5] = [/; /^    "k_b_utf8",$/d' src/codegen.rs
grep -qxF 'pub const PRESERVE_NONE_DOORS: [&str; 5] = [' src/codegen.rs
if grep -qxF "$door" src/codegen.rs; then exit 1; fi
