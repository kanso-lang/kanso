#!/bin/sh
# The emitter's arm ladder ranks a literal pattern below a bare binder, so a
# group whose literal arm is written first is compiled with the wildcard tried
# first. Native answers "any" where the oracle answers "zero".
#
# Arm order is the contract — arms are tried as written — and it is decided
# twice: once as the interpreter's arm search and once as this sort feeding the
# emitter's switch and guard chains. Which arm ANSWERS is what the dispatch
# sweep probes, and every probe returns a marker naming its arm so the
# difference reads as an arm rather than as a number.
set -e
target='Pattern::IntLit(..) | Pattern::StrLit(..) | Pattern::Nullary(..) => 3000,'
grep -qF "$target" src/codegen.rs || {
  echo "the arm ladder's literal rank moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's/Pattern::Nullary(..) => 3000,/Pattern::Nullary(..) => -1,/' src/codegen.rs
rm -f src/codegen.rs.bak
grep -qF 'Pattern::Nullary(..) => -1,' src/codegen.rs
