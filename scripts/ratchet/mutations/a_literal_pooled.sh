#!/bin/sh
# The pool takes only constants that run some of the program's work, so a
# literal moved ahead of the park is built where it is read. This mutation
# admits every constant: the literal goes through the pool's try, and the mem
# golden's `ahead_forced=1` goes to 2.
set -e
line='            .filter(|d| crate::ahead::admits(d))'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the pool's admission filter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            \.filter(|d| crate::ahead::admits(d))$/            .filter(|_| true)/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
