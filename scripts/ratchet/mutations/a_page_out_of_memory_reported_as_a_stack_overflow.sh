#!/bin/sh
# The playground reports a program that ran out of memory as one that ran out
# of stack.
#
# Both end in a trap that records nothing, and `kanso_take_rt_error` tells them
# apart by a flag the allocator sets when the page cannot grow. An allocator
# that no longer sets it sends every exhausted page to the stack sentence.
set -e
f=src/main.rs
grep -q 'note_out_of_memory' src/main.rs
grep -qF '            kanso::wasm::note_out_of_memory();' "$f" || {
  echo "the allocator's note moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^            kanso::wasm::note_out_of_memory();$/d' "$f"
! grep -q 'note_out_of_memory' "$f"
