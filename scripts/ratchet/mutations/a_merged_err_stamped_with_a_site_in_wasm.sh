#!/bin/sh
# The wasm engine stamps a merged err with the site of the operation that
# merged it.
#
# `rt_err_stamp` gives a fresh err the site it was born at. A merge also comes
# back without an origin, and the interpreter leaves it without one, so its
# report prints no birth site.
set -e
f=src/wasm_rt.rs
grep -q "fn rt_err_stamp" src/wasm_rt.rs
grep -qF 'Slot::V(Value::ErrV(info)) if info.origin.is_none() && !info.merged => {' "$f" || {
  echo "the merged test in rt_err_stamp moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/Slot::V(Value::ErrV(info)) if info.origin.is_none() \&\& !info.merged => {/Slot::V(Value::ErrV(info)) if info.origin.is_none() => {/' "$f"
grep -qF 'Slot::V(Value::ErrV(info)) if info.origin.is_none() => {' "$f"
