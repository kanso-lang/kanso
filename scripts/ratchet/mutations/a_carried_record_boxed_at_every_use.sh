#!/bin/sh
# A record carried in two words is boxed once and the box is reused by every
# later line in the same block, and everywhere when the box was made in the
# entry block. This mutation forgets each box as soon as it is made, so every
# line naming the record boxes it again, and the mem golden counts the extra
# records.
set -e
kept='        self.boxed_here.insert(e.to_string(), (t.to_string(), in_entry));'
[ "$(grep -cxF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the box cache moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        self\.boxed_here\.insert(e\.to_string(), (t\.to_string(), in_entry));$/        let _ = in_entry;/' src/codegen.rs
if grep -qxF "$kept" src/codegen.rs; then exit 1; fi
