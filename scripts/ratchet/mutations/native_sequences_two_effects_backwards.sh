#!/bin/sh
# The first step of a bind never runs on the interpreter. `io/write "a" .>
# (_ -> io/write "b")` prints `b` there where native prints `ab`.
#
# The executor is written twice — the interpreter's and the C runtime's — with
# no shared code between them, and only the effects sweep asks what happens
# when two effects are SEQUENCED rather than what one of them returns. A bind
# runs its subject before its callback, which is the whole content of the rule
# this breaks. The mutation sits in the interpreter because the sweep itself
# runs native: breaking the C side breaks the harness before it can compare.
set -e
f=src/eval.rs
grep -q 'fn execute_chain' src/eval.rs
anchor='                Desc::Bind(inner, callee) => {'
line='                    let yielded = self.execute(inner, executor)?;'
[ "$(grep -cxF "$anchor" "$f")" -eq 1 ] || {
  echo "the interpreter's bind arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i "/^                Desc::Bind(inner, callee) => {\$/{n;s/^                    let yielded = self.execute(inner, executor)?;\$/                    let yielded = { let _ = inner; Value::Done };/}" "$f"
grep -qxF '                    let yielded = { let _ = inner; Value::Done };' "$f"
