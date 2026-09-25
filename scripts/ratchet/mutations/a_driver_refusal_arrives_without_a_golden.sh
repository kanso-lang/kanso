#!/bin/sh
# The same rule as a_diagnostic_arrives_without_a_golden, on the other opener.
#
# The loader and the driver do not build a Diagnostic. They write `error: …`
# as plain text, print it, and exit — thirty-one sites in src/, and until the
# scan learned that second opener it walked past every one of them. A message
# a user meets before anything is compiled could be reworded or lost with
# nothing going red.
#
# So the driver arm is a thing the gate depends on, and this holds it there.
# The bait is an `error:` write with a run past the ten-character floor and no
# golden anywhere: it goes unnoticed the moment the arm is removed, or the
# moment its corpus is widened until anything matches.
set -e
grep -qF 'error: main is not an io' src/main.rs || {
  echo "the driver's plan refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|            eprintln!("error: main is not an io|            eprintln!("error: this driver refusal has no golden anywhere");\n            eprintln!("error: main is not an io|' src/main.rs
rm -f src/main.rs.bak
grep -qF 'this driver refusal has no golden anywhere' src/main.rs
