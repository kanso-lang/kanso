#!/bin/sh
# The wasm engine answers text no other engine answers.
#
# `kanso_exec_main` is the browser's entry point: it runs the compiled
# program's main and hands the text back through the output buffer. It is
# behind `#[cfg(target_arch = "wasm32")]`, so a change here reaches the engine
# in the page and no other, which is what makes it a differential defect rather
# than a change of behaviour everywhere at once.
#
# Appending one byte to every answer is loud on purpose: the sweep compares
# byte for byte against native over the whole golden corpus, so a defect that
# reaches every program should be reported for every program.
set -e
f=src/wasm.rs
# THE PATH IS SPELLED ON A GUARD LINE, and that is load-bearing rather than
# decorative. The ratchet's `touched` pass selects the rows a branch could have
# made blind by intersecting the files a branch changed with the paths each
# mutation names on a guard line. Every assertion below reaches the file
# through "$f", so the path appeared on no guard line and this row was
# invisible to that pass whatever a branch touched -- the same defect
# kanso#1338 repaired on the two compile-path mutations.
grep -q 'kanso_exec_main' src/wasm.rs
grep -qF '    let (status, text) = crate::wasm_rt::exec_main(h);' "$f" || {
  echo "the wasm entry point moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    set_out(&text);$|    set_out(\&format!("{text}x"));|' "$f"
grep -qF 'set_out(&format!("{text}x"));' "$f"
