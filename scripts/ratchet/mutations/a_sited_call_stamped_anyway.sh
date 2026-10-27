#!/bin/sh
# A twin that takes its call's site stores it only on the branch that reaches
# the C, so the call is not stamped first. This mutation stamps it anyway, at
# every such call. The output is the same; the witness is the work vein.
set -e
line='                true => args_ir.push(self.origin_arg(f, span)),'
grep -qxF "$line" src/codegen/calls.rs || {
  echo "the sited call's argument moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                true => args_ir.push(self.origin_arg(f, span)),$/                true => { self.stamp_site(f, span); args_ir.push(self.origin_arg(f, span)) }/' src/codegen/calls.rs
if grep -qxF "$line" src/codegen/calls.rs; then exit 1; fi
