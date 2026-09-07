#!/bin/sh
# Native's integer `%` answers a negative operand differently from the oracle,
# which truncates: this mutation makes the emitter write `urem` where it
# writes `srem`, so `-7 % 3` reads the two's-complement of -7 as a large
# positive and answers 0 where the interpreter answers -1. Both truncation and
# flooring are a language's defensible choice -- C and Rust truncate, Python
# and Haskell floor -- and kanso may only have one of them.
#
# Until 2026-09-07 this patched the runtime's k_mod to floor. kanso#1292 made
# the emitter write the remainder as one instruction for two proved integers
# and send only the zero and minus-one divisors to the call, so the runtime
# arm stopped answering the sweep's cases and the mutation went BLIND: the
# gate stayed green with the floor in place. The operator's answer now lives
# in the emitter, so that is what the mutation reaches.
#
# Nothing else in the board can see this. Every allocation counter is flat, no
# diagnostic is raised so `diagnostic_differential` has nothing to compare, and
# no golden in the corpus prints a negative modulo. What differs is an ANSWER,
# on one operator, at the operands the numeric sweep exists to straddle.
set -e
target='let insn = if op == "%" { "srem" } else { "sdiv" };'
grep -qF "$target" src/codegen.rs || {
  echo "the inline remainder moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|let insn = if op == "%" { "srem" } else { "sdiv" };|let insn = if op == "%" { "urem" } else { "sdiv" };|' \
  src/codegen.rs
grep -qF 'let insn = if op == "%" { "urem" } else { "sdiv" };' src/codegen.rs
