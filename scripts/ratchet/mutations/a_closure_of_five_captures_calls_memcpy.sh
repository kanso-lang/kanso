#!/bin/sh
# k_closure copies up to eight captures as inline words since 2026-09-15,
# and hands a longer environment to k_copy_cold; before, five or more went
# to memcpy, a plain call that cost the closure five pushes on all 243,978
# of its calls a run. This puts the old shape back: four words inline and
# memcpy above that. Every environment holds the same values, so nothing but
# the work vein can see it, which is exactly what the row asserts.
set -e
grep -q "    if (ncaps <= 8) {" src/runtime.c || {
  echo "the closure's capture ladder changed shape; this mutation needs rewriting" >&2
  exit 1
}
grep -q "        k_copy_cold(env, caps, sizeof(KValue) \* ncaps);" src/runtime.c || {
  echo "the closure's long copy changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "s/    if (ncaps <= 8) {/    if (ncaps <= 4) {/" \
    -e "s/        k_copy_cold(env, caps, sizeof(KValue) \* ncaps);/        memcpy(env, caps, sizeof(KValue) * ncaps);/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "        memcpy(env, caps, sizeof(KValue) \* ncaps);" src/runtime.c
