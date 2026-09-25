#!/bin/sh
# Give every declaration its own copy of the module's path again, the way
# `decl.file = file.to_string()` did. The emitted program is identical either
# way — the path is only ever read — so the witness is the allocation vein.
#
# The anchor is the shared bind, which appears once. `Arc::clone(&file)` on its
# own does not: `std::sync::Arc::clone` is spelled at several sites now, and a
# guard on it would refuse for the wrong reason.
set -e
target='    let file: std::sync::Arc<str> = std::sync::Arc::from(file);'
n=$(grep -cF "$target" src/lib.rs)
[ "$n" -eq 1 ] || { echo "the shared bind moved or multiplied ($n); rewrite this" >&2; exit 1; }
awk '
  index($0, "decl.file = std::sync::Arc::clone(&file);") {
      print "        decl.file = std::sync::Arc::from(&*file);"
      next
  }
  { print }
' src/lib.rs > src/lib.rs.mut && mv src/lib.rs.mut src/lib.rs
