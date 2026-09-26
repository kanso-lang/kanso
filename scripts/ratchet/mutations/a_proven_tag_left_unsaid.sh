#!/bin/sh
# A parameter proven to be one kind of heap value is not said to be one.
#
# `assume_param_tag` hands LLVM the tag inference proved for a boxed
# parameter, which lets it fold the tag tests the inlined helpers make. With
# the test inverted, no program without a subtype gets an assume, every one of
# those tests comes back, and the benchmarks' instruction rows move.
set -e
f=src/codegen.rs
grep -q 'fn assume_param_tag(&self' src/codegen.rs
line='        if !self.sub_parents.is_empty()'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the assume's subtype test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        if !self.sub_parents.is_empty()$/        if self.sub_parents.is_empty()/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
