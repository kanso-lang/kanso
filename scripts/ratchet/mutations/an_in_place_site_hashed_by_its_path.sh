#!/bin/sh
# The interpreter asks whether a push writes in place against a set of the
# frame's own (line, column) sites, gathered once per frame. This mutation
# also asks the whole program's set, keyed by the file's path, on every
# call, which is what the question cost before. The answer is the same;
# interp_instructions is the witness. `joins_here` asks its own sites with
# the same line, so the edit is held to `writes_in_place`.
set -e
line='        sites.contains(&(span.line as usize, span.col as usize))'
inside=$(sed -n '/^    fn writes_in_place(/,/^    }$/p' src/eval.rs | grep -cxF "$line" || true)
[ "$inside" -eq 1 ] || {
  echo "the in-place question moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    fn writes_in_place(/,/^    }$/ s/^        sites.contains(&(span.line as usize, span.col as usize))$/        sites.contains(\&(span.line as usize, span.col as usize)) \&\& self.linearity.get().is_some_and(|(all, _)| all.contains(\&(site.file.clone(), span.line as usize, span.col as usize)))/' src/eval.rs
if sed -n '/^    fn writes_in_place(/,/^    }$/p' src/eval.rs | grep -qxF "$line"; then exit 1; fi
