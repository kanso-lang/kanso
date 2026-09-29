#!/bin/sh
# The wasm engine's failure check runs a lazy binding.
#
# A dispatcher asks `rt_is_failure` of every argument before it picks an arm.
# That check reads the registry without running a lazy binding, as the
# interpreter's check passes a thunk by. Reading through `slot` runs it, so a
# binding the chosen arm ignores fails the call anyway.
set -e
f=src/wasm_rt.rs
grep -q "fn rt_is_failure" src/wasm_rt.rs
awk '/pub extern "C" fn rt_is_failure/ {inside=1} inside && /match raw_slot\(h\) \{/ {found=1} inside && /^}/ {inside=0} END {exit !found}' "$f" || {
  echo "rt_is_failure no longer reads raw_slot; this mutation needs rewriting" >&2
  exit 1
}
awk '/pub extern "C" fn rt_is_failure/ {inside=1} inside && /match raw_slot\(h\) \{/ {sub(/raw_slot/, "slot")} inside && /^}/ {inside=0} {print}' "$f" > "$f.mut"
mv "$f.mut" "$f"
awk '/pub extern "C" fn rt_is_failure/ {inside=1} inside && /match slot\(h\) \{/ {found=1} inside && /^}/ {inside=0} END {exit !found}' "$f"
