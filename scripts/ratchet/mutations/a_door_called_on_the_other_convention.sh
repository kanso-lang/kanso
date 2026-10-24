#!/bin/sh
# The runtime's doors are defined with K_DOORCC and called with
# preserve_nonecc. This mutation takes k_b_utf8 off the emitter's list, so the
# runtime defines it on one convention and the program calls it on the
# other; the witness is the_doors_the_program_calls_agree. The list's length
# is read rather than written here, so a door added or retired elsewhere does
# not stale the row.
set -e
n=$(sed -n 's/^pub const PRESERVE_NONE_DOORS: \[&str; \([0-9]*\)\] = \[$/\1/p' src/codegen/module.rs)
door='    "k_b_utf8",'
[ -n "$n" ] && [ "$(grep -cxF "$door" src/codegen/module.rs)" -eq 1 ] || {
  echo "the door list moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "s/^pub const PRESERVE_NONE_DOORS: \[&str; $n\] = \[\$/pub const PRESERVE_NONE_DOORS: [\&str; $((n - 1))] = [/; /^    \"k_b_utf8\",\$/d" src/codegen/module.rs
grep -qxF "pub const PRESERVE_NONE_DOORS: [&str; $((n - 1))] = [" src/codegen/module.rs
if grep -qxF "$door" src/codegen/module.rs; then exit 1; fi
