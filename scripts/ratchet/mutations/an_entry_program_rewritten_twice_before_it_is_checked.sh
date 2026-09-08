#!/bin/sh
# Restore the four rewrite passes `compile_parsed_entry` used to run before
# its whole-program check and again after it. The emitted text is identical
# either way — the passes are idempotent on their own output — so the only
# witness a repeated rewrite leaves is the pass count, and
# tests/rewrite_passes.rs is what reads it.
#
# THE ANCHOR IS THE CHECK LINE. `check::check_merged` is called four times in
# src/lib.rs and `finish_program` many more, so neither is a guard that can
# refuse. `check::check_merged(&merged, true)` appears exactly once — the
# other three pass `require_entry` or `false` — and the count is asserted
# before anything is inserted.
set -e
target='    let merged_diags = check::check_merged(&merged, true);'
n=$(grep -cF "$target" src/lib.rs)
[ "$n" -eq 1 ] || { echo "the entry check moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk '
  { print }
  index($0, "let merged_diags = check::check_merged(&merged, true)") {
      print "    finish_program(&mut merged);"
      print "    phase::watched(\"desugar_field_reads\", || desugar_field_reads(&mut merged));"
      print "    phase::watched(\"prune_unused_getters\", || prune_unused_getters(&mut merged));"
      print "    trmc::rewrite(&mut merged);"
  }
' src/lib.rs > src/lib.rs.mut && mv src/lib.rs.mut src/lib.rs
