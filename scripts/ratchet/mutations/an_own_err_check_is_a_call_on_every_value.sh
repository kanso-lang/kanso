#!/bin/sh
# Since 2026-09-15 the own-err check in front of an err-admitting arm is an
# alwaysinline twin: the tag test happens in the caller, and only an actual
# err reaches k_not_own_err's package compare. Before, every value matched
# against such an arm crossed the call: 1,454,508 calls a run on the run
# program, nearly all of them on values that were never an err. This puts
# the bare call back at both sites the emitter writes it.
set -e
n=$(grep -cF 'call i64 @k_not_own_err_fast(%KValue {value}, ptr @{arm})' src/codegen.rs)
[ "$n" -eq 2 ] || { echo "the own-err check sites changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|call i64 @k_not_own_err_fast(%KValue {value}, ptr @{arm})|call i64 @k_not_own_err(%KValue {value}, ptr @{arm})|' src/codegen.rs
grep -qF 'call i64 @k_not_own_err(%KValue {value}, ptr @{arm})' src/codegen.rs
