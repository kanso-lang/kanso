#!/bin/sh
# An inline length read without its bound. A loop that guards with
# `length xs < i` and then steps `i + 1` takes its overflow test back, and
# the emitted goldens read the assumes that went.
set -e
old='                assume_length(f, &len);'
[ "$(grep -cxF "$old" src/codegen/emit.rs)" -eq 1 ] || {
  echo "the inline length's bound moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^                assume_length(f, &len);$/d' src/codegen/emit.rs
! grep -qxF "$old" src/codegen/emit.rs
