#!/bin/sh
# The demand analyser claims every binding is demanded, which erases the lazy
# tier: no thunk is ever emitted, and pendbench's thunk counters — the only
# nonzero ones in the corpus — read zero. This is the strictness-analysis
# change the pending-cell golden exists to catch: two honest benchmark drafts
# read zero this way and looked like broken fixtures rather than a working
# analyser.
set -e
A='        self.lazy_binds.contains(&(fn_name, arity, stmt_index))' \
awk '
  !hit && $0 == ENVIRON["A"] {
    print "        let _ = (fn_name, arity, stmt_index);"
    print "        false"
    hit = 1
    next
  }
  { print }
  END { if (!hit) exit 3 }
' src/demand.rs > src/demand.rs.mut || {
  rm -f src/demand.rs.mut
  echo "the lazy-bind membership moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/demand.rs.mut src/demand.rs
grep -q 'let _ = (fn_name, arity, stmt_index);' src/demand.rs || exit 1
exit 0
