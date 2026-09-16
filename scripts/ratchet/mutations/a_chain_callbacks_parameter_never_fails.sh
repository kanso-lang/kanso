#!/bin/sh
# Inference seeds every lambda parameter as never failing, because an
# ordinary call refuses to hand a closure a failure. `rescue` and `annotate`
# hand their callback the failure itself, so their callback lambda is walked
# with a parameter that may hold an err. This mutation seeds that parameter
# like every other lambda's, which is what the emitter used to be told: the
# group the callback hands the err on to then loses its entry guard on
# native and takes the err into its body, and the micro corpus reads native
# against the interpreter on a_chain_callback_is_handed_the_err_itself.
set -e
grep -q '^        inner.insert(p, TOP);$' src/infer.rs || {
  echo "the callback walker's seed changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^        inner.insert(p, TOP);$@        inner.insert(p, TOP \& !FAIL);@' src/infer.rs
grep -q '^        inner.insert(p, TOP & !FAIL);$' src/infer.rs
