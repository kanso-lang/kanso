#!/bin/sh
# Every checked integer op in a function branches to one overflow trap, which
# `body` writes once at the end. This mutation stops the ops reusing it, so
# each takes a trap of its own and the function carries a copy an op. The
# output is the same; the witness is an_overflow_trap_is_written_once_a_function.
set -e
line='        if let Some(trap) = self.overflow_traps.first() {'
[ "$(grep -cF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the shared overflow trap moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if let Some(trap) = self.overflow_traps.first() {$/        if let Some(trap) = None::<\&String> {/' src/codegen.rs
[ "$(grep -cF '        if let Some(trap) = None::<&String> {' src/codegen.rs)" -eq 1 ]
