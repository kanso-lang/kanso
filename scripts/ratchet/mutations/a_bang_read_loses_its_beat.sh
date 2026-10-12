#!/bin/sh
# A chain head that names a declaration the program made stops being asked what
# that declaration yields, and falls to the top set instead. That was the hole
# readbench was built for: `os/read_file!` is a declared wrapper, and inference
# used to answer a chain's yield from a table keyed on the head's BARE name, so
# `os/read_file` hit the table and `os/read_file!` did not.
#
# It edits the consult rather than the golden, because the golden is what an
# edit would prove nothing about.
#
# readbench no longer sees it (2026-10-12). Its loop carries nothing across the
# rewind, so the beat analysis never asks the document's type, and the mutation
# leaves every counter where it was. oneshot reads the same `os/read_file!`
# chain and still does: on this mutation its cohort_kept reads 0 against 1.
set -e
A='                if let Some(y) = group_yield(ctx, n.as_str(), args.len()) {'
B='                    return y;'
grep -qF "$A" src/infer.rs && grep -qF "$B" src/infer.rs || {
  echo "the group consult moved; this mutation needs rewriting" >&2
  exit 1
}
A="$A" B="$B" awk '
  $0 == ENVIRON["A"] { print "                if group_yield(ctx, n.as_str(), args.len()).is_some() {"; next }
  $0 == ENVIRON["B"] { print "                    return TOP & !FAIL;"; next }
  { print }
' src/infer.rs > src/infer.rs.mut
mv src/infer.rs.mut src/infer.rs
grep -qF 'group_yield(ctx, n.as_str(), args.len()).is_some()' src/infer.rs
