#!/bin/sh
# A cluster of groups in one tail cycle goes back to being disqualified by any
# tail edge from outside it. The mem fixture that pins the demotion then reads
# nine arena blocks where its golden says one, and beat_iters zero where the
# golden says 400,001.
#
# It restores the old refusal rather than editing the golden, because an edit
# would prove nothing about the counters. A fixture whose numbers came from
# somewhere other than the bracket survives an edited golden and dies here.
set -e
python3 - <<'PY'
path = "src/beat.rs"
line = "            if entries.iter().any(|(from, _)| inside.contains(groups[*from].0.as_str())) {\n"
source = open(path).read()
assert line in source, "the cluster entry test moved; this mutation needs rewriting"
open(path, "w").write(source.replace(line, "            if true {\n", 1))
PY
grep -q 'inside.contains(groups\[\*from\]' src/beat.rs && exit 1
exit 0
