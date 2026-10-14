#!/bin/sh
# A release build stops keeping tailcc for any arm at all, on either
# architecture: every arm takes the C convention and every tail call becomes
# an ordinary call, which is what the release build did before the register
# cap was measured.
set -e
x86='        (true, _) => preserve_none_calls(preserve_none_tails(ir), include_str!("runtime.c")),'
apple='        (false, true) if cfg!(target_vendor = "apple") => narrow_tailcc(ir, ARM64_TAILCC_WIDEST),'
arm='        (false, true) => wide_tailcc_keeps_its_jumps(ir, ARM64_TAILCC_WIDEST),'
for line in "$x86" "$apple" "$arm"; do
  [ "$(grep -cxF "$line" src/main.rs)" -eq 1 ] || {
    echo "the release rewrite's call sites moved; this mutation needs rewriting" >&2
    exit 1
  }
done
sed -i 's/^        (true, _) => preserve_none_calls(preserve_none_tails(ir), include_str!("runtime.c")),$/        (true, _) => preserve_none_calls(preserve_none_tails(narrow_tailcc(ir, 0)), include_str!("runtime.c")),/' src/main.rs
sed -i 's/^        (false, true) if cfg!(target_vendor = "apple") => narrow_tailcc(ir, ARM64_TAILCC_WIDEST),$/        (false, true) if cfg!(target_vendor = "apple") => narrow_tailcc(ir, 0),/' src/main.rs
sed -i 's/^        (false, true) => wide_tailcc_keeps_its_jumps(ir, ARM64_TAILCC_WIDEST),$/        (false, true) => narrow_tailcc(ir, 0),/' src/main.rs
if grep -qxF "$x86" src/main.rs || grep -qxF "$apple" src/main.rs || grep -qxF "$arm" src/main.rs; then exit 1; fi
