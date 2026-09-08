#!/bin/sh
# Restore the four rewrite passes `compile_parsed_entry` used to run before
# its whole-program check and again after it. The emitted text is identical
# either way — the passes are idempotent on their own output — so the only
# witness a repeated rewrite leaves is the pass count, and
# tests/rewrite_passes.rs is what reads it.
#
# THE RESTORED GROUP CARRIES ITS COUNTERS, because `kanso::rewrite` counts at
# the call sites in `compile_parsed_entry` rather than inside the four
# functions. It sits there because `compile_module_loaded` calls the same four
# and that path is what the compile gates measure: counting inside them put 502
# instructions on every module compile, and CI's trend gate refused it as a
# pure regression. So this mutation writes what somebody adding the group back
# would write — the calls and their bumps — and tests/rewrite_passes.rs says
# plainly that a pass added without a bump is one this spec cannot see.
#
# THE ANCHOR IS THE CHECK LINE. `check::check_merged` is called four times in
# src/lib.rs and `finish_program` many more, so neither is a guard that can
# refuse. Since kanso#1335 the entry path calls `check_merged_after_aliases`
# instead — the alias pass runs in front of it now and hands it the record —
# and that name appears exactly once in the file, which is what makes it a
# usable anchor. The count is asserted before anything is inserted.
set -e
target='    let merged_diags = check::check_merged_after_aliases(&merged, true, &rewritten);'
n=$(grep -cF "$target" src/lib.rs)
[ "$n" -eq 1 ] || { echo "the entry check moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk '
  { print }
  index($0, "let merged_diags = check::check_merged_after_aliases(&merged, true, &rewritten)") {
      print "    rewrite::pass();"
      print "    finish_program(&mut merged);"
      print "    rewrite::pass();"
      print "    phase::watched(\"desugar_field_reads\", || desugar_field_reads(&mut merged));"
      print "    rewrite::pass();"
      print "    phase::watched(\"prune_unused_getters\", || prune_unused_getters(&mut merged));"
      print "    rewrite::pass();"
      print "    trmc::rewrite(&mut merged);"
  }
' src/lib.rs > src/lib.rs.mut && mv src/lib.rs.mut src/lib.rs
