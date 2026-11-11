#!/bin/sh
# The interpreter's tables are indexed by numbers the program decides. This
# mutation lets an address back in: the table of callees is not asked for a
# name whose text sits where bit 24 of its address is set, so which names
# are looked up again on every call depends on where valgrind placed the
# heap, and the interpreted run's row reads apart with the mappings started
# at 48 MiB. The gate compares the two readings.
set -e
grep -c 'self.callees_by_number.borrow().get(at as usize)' src/eval/calls.rs | grep -qx 1 || {
  echo "callee_of_ref's read of its table moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/self.callees_by_number.borrow().get(at as usize)/self.callees_by_number.borrow().get(at as usize + ((Rc::as_ptr(name) as *const u8 as usize >> 24) \& 1) * (usize::MAX \/ 2))/' src/eval/calls.rs
grep -q 'as usize >> 24) & 1) \* (usize::MAX / 2))' src/eval/calls.rs
