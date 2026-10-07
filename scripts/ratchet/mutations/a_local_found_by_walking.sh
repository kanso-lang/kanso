#!/bin/sh
# A local reference keeps the frame and slot it found its binding in, and
# later runs read that place. This mutation keeps plain local instead, so
# every reference walks the frames again. The answers are the same; the
# interpreted row is the witness.
set -e
kept='(d, s) if d < 128 && s < 256 => 3 + (d << 8) + s as u32,'
[ "$(grep -cF "$kept" src/eval.rs)" -eq 1 ] || {
  echo "the kept place moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/(d, s) if d < 128 \&\& s < 256 => 3 + (d << 8) + s as u32,/(d, s) if d < 128 \&\& s < 256 \&\& false => 3 + (d << 8) + s as u32,/' src/eval.rs
if grep -qF "$kept" src/eval.rs; then exit 1; fi
