#!/bin/sh
# A release build stops keeping tailcc for any arm at all, on either
# architecture: every arm takes the C convention and every tail call becomes
# an ordinary call, which is what the release build did before the register
# cap was measured.
set -e
x86='        (true, _) => preserve_none_calls(preserve_none_tails(ir), kanso::RUNTIME_C),'
arm='        (false, true) => arm64_tails(ir),'
for line in "$x86" "$arm"; do
  [ "$(grep -cxF "$line" src/main.rs)" -eq 1 ] || {
    echo "the release rewrite's call sites moved; this mutation needs rewriting" >&2
    exit 1
  }
done
plain='ir.replace("musttail ", "").replace("tailcc ", "")'
sed -i "s/^        (true, _) => preserve_none_calls(preserve_none_tails(ir), kanso::RUNTIME_C),\$/        (true, _) => preserve_none_calls(preserve_none_tails($plain), kanso::RUNTIME_C),/" src/main.rs
sed -i "s/^        (false, true) => arm64_tails(ir),\$/        (false, true) => $plain,/" src/main.rs
if grep -qxF "$x86" src/main.rs || grep -qxF "$arm" src/main.rs; then exit 1; fi
