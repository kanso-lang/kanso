#!/bin/sh
# The wasm engine reads a lazy binding as the closure that holds it.
#
# `slot` runs a lazy binding on its first read and writes the answer over it.
# Without that, a read finds the closure, and the binding never answers the
# value it names.
set -e
f=src/wasm_rt.rs
grep -q "fn demanded_binding" src/wasm_rt.rs
grep -qF '        Slot::C { tidx, env, arity: LAZY } => demanded_binding(h, tidx, env),' "$f" || {
  echo "the lazy read in slot moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        Slot::C { tidx, env, arity: LAZY } => demanded_binding(h, tidx, env),$/        Slot::C { tidx, env, arity: LAZY } if false => demanded_binding(h, tidx, env),/' "$f"
grep -qF 'arity: LAZY } if false =>' "$f"
