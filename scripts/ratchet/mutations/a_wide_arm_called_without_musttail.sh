#!/bin/sh
# A release build hands clang every `musttail` the emitter wrote. An arm wider
# than the twelve words `preserve_nonecc` passes in registers keeps `tailcc`,
# and this mutation takes `musttail` off every call into such an arm, as the
# release build's narrowing did until 2026-10-12. A self call through a
# thirteen-word arm that builds a record each hop then runs out of stack.
set -e
line='        (true, _) => preserve_none_calls(preserve_none_tails(ir), kanso::RUNTIME_C),'
[ "$(grep -cxF "$line" src/main.rs)" -eq 1 ] || {
  echo "the release rewrite's call site moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        (true, _) => preserve_none_calls(preserve_none_tails(ir), kanso::RUNTIME_C),$/        (true, _) => preserve_none_calls(preserve_none_tails(ir).replace("musttail call tailcc ", "call tailcc "), kanso::RUNTIME_C),/' src/main.rs
if grep -qxF "$line" src/main.rs; then exit 1; fi
