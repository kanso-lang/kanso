#!/bin/sh
# A group on a cycle that grows the stack records the cycle's place on entry,
# which a stack overflow names (src/deep.rs). This mutation stops recording it
# on native, so the overflow goldens lose their `failed in` line there.
set -e
line='        f.line_fmt(format_args!("store ptr @{lit}, ptr @k_deep_site"));'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the deep-site store moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        f.line_fmt(format_args!("store ptr @{lit}, ptr @k_deep_site"));$/        let _ = lit;/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
