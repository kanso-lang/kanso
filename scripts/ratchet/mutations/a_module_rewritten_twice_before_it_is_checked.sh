#!/bin/sh
# `compile_module_loaded` used to run finish_program, desugar_field_reads,
# prune_unused_getters and trmc::rewrite between `check_merged` and the diags
# render, and then run all four AGAIN after canonicalize_types,
# canonicalize_bare_aliases, hoist_repeated_strings and fuse_enumerable. On the
# success path that is four whole-program passes done twice; on the error path
# the whole lot is done and thrown away, because the render reads only `diags`.
# Since 2026-09-08 only the second block survives. This mutation puts the first
# one back. The emitted code is identical either way -- emitted_code AGREED
# when the block was removed -- so the compile-instructions vein is the witness.
#
# The anchor is the check_merged line rather than the inline call below it:
# `inline::inline_builtin_wrappers(&mut merged)` also appears in the
# single-file compile paths, and a sed on that name patched the wrong site.
set -e
target='    let diags = phase::watched("check_merged", || check::check_merged(&merged, require_entry));'
n=$(grep -cF "$target" src/lib.rs)
[ "$n" -eq 1 ] || { echo "the merged check moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk '
  { print }
  index($0, "let diags = phase::watched(\"check_merged\"") {
      print "    finish_program(&mut merged);"
      print "    phase::watched(\"desugar_field_reads\", || desugar_field_reads(&mut merged));"
      print "    phase::watched(\"prune_unused_getters\", || prune_unused_getters(&mut merged));"
      print "    trmc::rewrite(&mut merged);"
  }
' src/lib.rs > src/lib.rs.mut && mv src/lib.rs.mut src/lib.rs
grep -qF '    trmc::rewrite(&mut merged);
    inline::inline_builtin_wrappers(&mut merged);' src/lib.rs
