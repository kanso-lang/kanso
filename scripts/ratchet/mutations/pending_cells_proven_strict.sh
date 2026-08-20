#!/bin/sh
# The demand analyser claims every binding is demanded, which erases the lazy
# tier: no thunk is ever emitted, and pendbench's thunk counters — the only
# nonzero ones in the corpus — read zero. This is the strictness-analysis
# change the pending-cell golden exists to catch: two honest benchmark drafts
# read zero this way and looked like broken fixtures rather than a working
# analyser.
set -e
python3 - <<'PY'
path = "src/demand.rs"
line = "        self.lazy_binds.contains(&(fn_name.to_string(), arity, stmt_index))\n"
source = open(path).read()
assert line in source, "the lazy-bind membership moved; this mutation needs rewriting"
open(path, "w").write(source.replace(line, "        let _ = (fn_name, arity, stmt_index);\n        false\n", 1))
PY
grep -q 'let _ = (fn_name, arity, stmt_index);' src/demand.rs || exit 1
exit 0
