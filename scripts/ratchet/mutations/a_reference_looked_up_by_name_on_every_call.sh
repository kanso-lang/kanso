#!/bin/sh
# callee_of_ref reads its table past the end, so the table never answers and
# every call through a reference hashes the name's text again (kanso#2444).
# Every program still prints what it printed.
set -e
grep -c 'self.callees_by_number.borrow().get(at as usize)' src/eval/calls.rs | grep -qx 1 || {
  echo "callee_of_ref's read of its table moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/self.callees_by_number.borrow().get(at as usize)/self.callees_by_number.borrow().get(usize::MAX)/' src/eval/calls.rs
grep -q 'self.callees_by_number.borrow().get(usize::MAX)' src/eval/calls.rs
