#!/bin/sh
# The same rule again, on the third opener.
#
# `error[kind]: …` is what a rendered Diagnostic looks like on a terminal, so
# a message written that way as plain text reads to a user exactly like one the
# corpus pins — and the scan, keyed first on the constructor and then on
# `error: `, saw neither. Twenty-odd sites: the runtime's endpoints, the
# stack-depth refusal, the exit-code refusals, the repl's name lookups.
#
# This holds the third arm there. The bait is an `error[` write with a run past
# the ten-character floor and no golden anywhere.
set -e
grep -qF 'error[endpoint]: unhandled none reached the entry' src/main.rs || {
  echo "the endpoint refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|            eprintln!("error\[endpoint\]: unhandled none reached the entry");|            eprintln!("error[wandering]: this printed refusal has no golden");\n            eprintln!("error[endpoint]: unhandled none reached the entry");|' src/main.rs
rm -f src/main.rs.bak
grep -qF 'this printed refusal has no golden' src/main.rs
