#!/bin/sh
# The interpreter asks whether a push writes in place against a set of the
# frame's own (line, column) sites, gathered once per frame. This mutation
# also asks the whole program's set, keyed by the file's path, on every
# call, which is what the question cost before. The answer is the same;
# interp_instructions is the witness.
set -e
line='        sites.contains(&(span.line as usize, span.col as usize))'
[ "$(grep -cxF "$line" src/eval.rs)" -eq 1 ] || {
  echo "the in-place question moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        sites.contains(&(span.line as usize, span.col as usize))$/        sites.contains(\&(span.line as usize, span.col as usize)) \&\& self.in_place.get().is_some_and(|all| all.contains(\&(site.file.clone(), span.line as usize, span.col as usize)))/' src/eval.rs
if grep -qxF "$line" src/eval.rs; then exit 1; fi
