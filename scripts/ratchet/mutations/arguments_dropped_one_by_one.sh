#!/bin/sh
# After a dispatch binds its arguments, what is left in the argument vector is
# let go through `release`, which skips `Value`'s drop for the variants that
# own nothing -- the `NoneV` placeholders `bind_moved` leaves among them. This
# mutation drops every one again. The answers are the same; the interpreted
# row is the witness.
set -e
kept='                        release(arg);'
[ "$(grep -cxF "$kept" src/eval/calls.rs)" -eq 1 ] || {
  echo "the released argument moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                        release(arg);$/                        drop(arg);/' src/eval/calls.rs
if grep -qxF "$kept" src/eval/calls.rs; then exit 1; fi
