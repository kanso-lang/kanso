#!/bin/sh
# The per-arm fold becomes a per-key disjunction, so one arm qualifying makes
# the whole group lazy again — the shape that had regexp's character-class arm
# thunking `at <= length s` once per character examined. Measured on the scan
# benchmark: thunk_allocs 0 becomes 501,502 and peak RSS rises about 92 MB.
#
# It edits the fold rather than the golden because the golden is what an edit
# would prove nothing about. A gate reading a counter no program moves, or one
# pinned to a benchmark that allocates no thunks at all, survives an edited
# golden and dies here.
set -e
python3 - <<'PY'
path = "src/demand.rs"
line = "            *seen = *seen && qualifies;\n"
source = open(path).read()
assert line in source, "the lazy fold moved; this mutation needs rewriting"
open(path, "w").write(source.replace(line, "            *seen = *seen || qualifies;\n", 1))
PY
grep -q '\*seen = \*seen && qualifies;' src/demand.rs && exit 1
exit 0
