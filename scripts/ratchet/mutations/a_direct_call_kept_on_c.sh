#!/bin/sh
# A release build on x86-64 with clang 19 gives every function the program
# calls only directly preserve_nonecc, so a callee saves nothing its caller
# does not keep. This mutation keeps every one on the C convention. The
# output is the same; the work vein is what sees it.
set -e
old='let kept = name == "main" || name.starts_with("k_") || externs.contains(&name);'
[ "$(grep -cF "$old" src/main.rs)" -eq 1 ] || {
  echo "the plain-call rewrite's test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/let kept = name == "main" || name.starts_with("k_") || externs.contains(&name);/let kept = true || externs.contains(\&name);/' src/main.rs
grep -qF 'let kept = true || externs.contains(&name);' src/main.rs
