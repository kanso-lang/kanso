#!/bin/sh
# A parameter that is a name holds `none` while the arms are tried, and the
# winner's argument is moved in afterwards. This mutation clones the argument
# at every candidate again, as match_one does. The answer is the same, since
# the move overwrites the clone; the interpreted run's instruction row sees it.
set -e
line='                binds.push((name.clone(), Value::NoneV));'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the held binding moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                binds.push((name.clone(), Value::NoneV));$/                binds.push((name.clone(), arg.clone()));/' src/eval.rs
grep -qxF '                binds.push((name.clone(), arg.clone()));' src/eval.rs
