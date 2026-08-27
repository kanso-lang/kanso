#!/bin/sh
# The same file, the other arm. `message: format!(` does not carry the opening
# quote — the literal may sit on the next line — so the message is the SECOND
# piece of a split on the quote where every other opener takes the first. A
# mutation on the `message: "` arm alone would leave that difference untested.
#
# The bait is the unknown-builtin refusal, which a program cannot reach:
# `builtin_nope 1` in user code is refused by name, and reaching this arm means
# a stdlib module naming a builtin that does not exist.
set -e
grep -qF 'message: format!("unknown builtin `{name}`")' src/eval.rs || {
  echo "the unknown-builtin refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|message: format!("unknown builtin `{name}`")|message: format!("this oracle refusal has no golden at all `{name}`")|' src/eval.rs
rm -f src/eval.rs.bak
grep -qF 'this oracle refusal has no golden at all' src/eval.rs
