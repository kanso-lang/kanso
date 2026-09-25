#!/bin/sh
# A release build on x86-64 with the preserve_none convention keeps a tail
# call into every arm of up to twelve argument words, the registers that
# convention passes in, so the JSON decoder's nine-word `obj_key_end` loop runs
# in one frame. This mutation narrows at eight, as arm64 must, and a
# 300,000-key object overflows the stack; the witness is
# a_big_object_decodes_in_a_release_build.
#
# It once changed `TAILCC_WIDEST`'s x86-64 value instead. Since the release
# path on a clang with preserve_none narrows at `PRESERVE_NONE_REGISTERS`,
# that value is read only where the probe finds no preserve_none, and the
# ratchet reported the row blind.
set -e
line='            preserve_none_tails(narrow_tailcc(ir, PRESERVE_NONE_REGISTERS)),'
[ "$(grep -cxF "$line" src/main.rs)" -eq 1 ] || {
  echo "the preserve_none narrowing moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            preserve_none_tails(narrow_tailcc(ir, PRESERVE_NONE_REGISTERS)),$/            preserve_none_tails(narrow_tailcc(ir, 8)),/' src/main.rs
if grep -qxF "$line" src/main.rs; then exit 1; fi
