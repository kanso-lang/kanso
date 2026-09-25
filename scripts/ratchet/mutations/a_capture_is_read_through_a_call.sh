#!/bin/sh
# Since 2026-09-15 a lambda reads each captured value straight out of its
# environment: a getelementptr and a load in the entry block, where before
# it called k_env_get, a three-instruction C function, once per capture on
# every entry. This puts the call back, through the same slot pointer so
# nothing else in the emitter changes shape. Every emitted lambda then
# carries one `call` per capture again, which the emitted vein counts and
# the work vein pays: 1,279,648 calls a run on the run program.
set -e
n=$(grep -cF '{t} = load %KValue, ptr {slot}' src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the capture load changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|{t} = load %KValue, ptr {slot}|{t} = call %KValue @k_env_get(ptr {slot}, i64 0)|' src/codegen.rs
grep -qF '{t} = call %KValue @k_env_get(ptr {slot}, i64 0)' src/codegen.rs
