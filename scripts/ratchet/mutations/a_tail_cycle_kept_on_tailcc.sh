#!/bin/sh
# A release build on x86-64 with clang 19 gives every tail cycle one flat
# signature under preserve_nonecc, so an arm that jumps to the next saves no
# callee-saved registers. This mutation skips the rewrite, the cycles keep
# tailcc, and the run program pays the pushes and pops again; the work vein
# is the only thing that can see it, since the output is the same.
set -e
line='            preserve_none_tails(narrow_tailcc(ir, PRESERVE_NONE_REGISTERS)),'
[ "$(grep -cxF "$line" src/main.rs)" -eq 1 ] || {
  echo "the rewrite's call site moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            preserve_none_tails(narrow_tailcc(ir, PRESERVE_NONE_REGISTERS)),$/            narrow_tailcc(ir, TAILCC_WIDEST),/' src/main.rs
[ "$(grep -cxF '            narrow_tailcc(ir, TAILCC_WIDEST),' src/main.rs)" -eq 1 ]
