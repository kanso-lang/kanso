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
grep -qF '    let (status, text) = crate::wasm_rt::exec_main(h);' "$f" || {
  echo "the wasm entry point moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    set_out(&text);$|    set_out(\&format!("{text}x"));|' "$f"
grep -qF 'set_out(&format!("{text}x"));' "$f"
