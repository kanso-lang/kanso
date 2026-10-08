#!/bin/sh
# The emitter writes each line of a function body with three pushes onto its
# buffer. This mutation sends the line through `writeln!` again, the way it
# went before 2026-10-08. The module is the same text; the emit row is the
# witness.
set -e
kept='        self.out.push_str("  ");'
[ "$(grep -cF "$kept" src/codegen.rs)" -eq 1 ] || {
  echo "the line write moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        self.out.push_str("  ");$/        let _ = writeln!(self.out, "  {text}"); return;/' src/codegen.rs
if grep -qF "$kept" src/codegen.rs; then exit 1; fi
