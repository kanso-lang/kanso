#!/bin/sh
# The decoder opens an object with the empty map literal, 273,339 times a run
# on runbench. Since 2026-09-25 the emitter calls k_map_empty for `{}`, which
# takes the header and a buffer of a constant class in one bump. This
# mutation turns that arm off, so `{}` goes through k_map_lit with a count of
# zero: a size class asked at run time, two bumps and a copy of nothing. The
# bytes out are the same either way, so the work vein is the witness.
set -e
line='            Expr::MapLit(pairs, _) if pairs.is_empty() => {'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the empty map literal's arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            Expr::MapLit(pairs, _) if pairs.is_empty() => {$/            Expr::MapLit(pairs, _) if false \&\& pairs.is_empty() => {/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
