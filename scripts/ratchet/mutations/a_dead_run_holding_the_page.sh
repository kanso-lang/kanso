#!/bin/sh
# The browser engine keeps the last run's values until after the next compile.
#
# `kanso_play_wasm` and `kanso_compile_wasm` release the registry before they
# compile. Without that, a run that filled the page leaves the next compile
# nowhere to allocate, and the engine traps on every program after it.
set -e
f=src/wasm.rs
grep -q 'wasm_rt::release' src/wasm.rs
grep -qF '    crate::wasm_rt::release();' "$f" || {
  echo "the release before a compile moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    crate::wasm_rt::release();$/d' "$f"
! grep -q 'wasm_rt::release' "$f"
