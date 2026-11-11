#!/bin/sh
# Inline text/to_bytes into its caller again, as the compiler did until
# 2026-09-29. Its err is then born in the caller's module, a rescue there hands
# it on as its own, and the micro corpus goes red.
#
# The builtins whose wrappers stay are one list, `BIRTHS_ERR`, which the
# emitter reads as well, so the mutation leaves the list alone and makes the
# inliner pass over one name in it.
set -e
grep -q 'BIRTHS_ERR.contains(&bare)' src/inline.rs || {
  echo "the inliner's test for a builtin that can bear an err moved; this mutation needs rewriting" >&2
  exit 1
}
old='        if target.strip_prefix("builtin_").is_some_and(|bare| BIRTHS_ERR.contains(&bare)) {'
[ "$(grep -cxF "$old" src/inline.rs)" -eq 1 ]
sed -i 's/is_some_and(|bare| BIRTHS_ERR\.contains(&bare)) {$/is_some_and(|bare| bare != "to_bytes" \&\& BIRTHS_ERR.contains(\&bare)) {/' src/inline.rs
grep -q 'bare != "to_bytes" && BIRTHS_ERR.contains(&bare)' src/inline.rs
