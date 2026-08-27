#!/bin/sh
# The fifth opener, on the engine the differential law calls the truth.
#
# `src/eval.rs` raises its refusals as `RuntimeError { message: ... }`, a bare
# struct field matching none of the four openers before it, so the oracle could
# reword any of its 97 messages with nothing going red.
#
# This holds the `message: "` arm, where the literal follows the opener
# directly. The bait is the bind-pattern catch-all, which the parser makes
# unreachable — `_ = ...` and `7 = ...` are both `expected a binding name or
# type` — so the mutation cannot change what any fixture prints.
set -e
grep -qF 'message: "binding patterns are irrefutable: names and constructor \' src/eval.rs || {
  echo "the bind-pattern refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|message: "binding patterns are irrefutable: names and constructor \\|message: "this oracle refusal has no golden at all, none \\|' src/eval.rs
rm -f src/eval.rs.bak
grep -qF 'this oracle refusal has no golden at all' src/eval.rs
