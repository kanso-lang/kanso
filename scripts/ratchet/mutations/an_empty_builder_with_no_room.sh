#!/bin/sh
# `bytes ""` is a view of the empty string again.
#
# The emitter writes the literal as a call to k_b_bytes_seed, a builder with
# 64 bytes of room, so the append that follows it lands in place. Without the
# arm the literal takes the ordinary view and its first append grows from
# nothing, and the mem vein reads the malloc'd buffers again.
set -e
f=src/codegen.rs
grep -q 'k_b_bytes_seed' src/codegen.rs
line='        if first.is_none() && args.len() == 1 && self.builtin_named(name, 1) == "bytes" {'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the seeded builder's arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if first.is_none() \&\& args.len() == 1 \&\& self.builtin_named(name, 1) == "bytes" {$/        if false \&\& first.is_none() \&\& args.len() == 1 \&\& self.builtin_named(name, 1) == "bytes" {/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
