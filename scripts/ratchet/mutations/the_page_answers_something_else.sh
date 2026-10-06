#!/bin/sh
# The page answers a sentence no other engine answers.
#
# A program that runs out of stack on the page ends in a wasm trap, and the
# page asks the toolchain for the sentence native prints, through
# `kanso_stack_exhausted`. That export is behind `#[cfg(target_arch =
# "wasm32")]`, so a change here reaches the engine in the page and no other,
# which makes it a differential defect rather than a change of behaviour
# everywhere at once. The corpus carries a program that runs out of stack.
set -e
f=src/wasm.rs
# The path is spelled on a guard line so the `touched` pass can see which
# file this mutation reaches; every assertion below goes through "$f".
grep -q 'fn kanso_stack_exhausted' src/wasm.rs
grep -qF '    set_out(&format!("{}\n", crate::stack_exhausted()));' "$f" || {
  echo "the page's stack sentence moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    set_out(&format!("{}\\n", crate::stack_exhausted()));$|    set_out(\&format!("{}!\\n", crate::stack_exhausted()));|' "$f"
grep -qF 'set_out(&format!("{}!\n", crate::stack_exhausted()));' "$f"
