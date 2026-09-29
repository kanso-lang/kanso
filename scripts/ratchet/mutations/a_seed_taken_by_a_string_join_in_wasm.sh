#!/bin/sh
# The wasm engine lets a string join take a builder it does not own.
#
# `rt_template_mut` takes the builder out of its handle only when a previous
# join handed that handle straight to the call. A seed is not taken: a literal
# lives in one handle that every mention shares, so taking it empties the
# literal for the rest of the run.
set -e
f=src/wasm_rt.rs
grep -q 'fn rt_template_mut' src/wasm_rt.rs
grep -qF '    let owned = OWNED.with(|o| o.borrow_mut().remove(&first));' "$f" || {
  echo "the ownership test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    let owned = OWNED.with(|o| o.borrow_mut().remove(&first));$/    let owned = OWNED.with(|o| o.borrow_mut().remove(\&first)) || true;/' "$f"
grep -qF 'remove(&first)) || true;' "$f"
