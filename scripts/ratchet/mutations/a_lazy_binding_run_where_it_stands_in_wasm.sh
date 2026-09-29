#!/bin/sh
# The wasm engine runs a lazy binding where it stands.
#
# `emit_decl_body` makes a binding `demand` marked lazy into a closure its
# first read runs. Asking about no binding sends every body down the strict
# path, so a binding that fails dies even when nothing reads it.
set -e
f=src/wasm_backend.rs
grep -q "fn emit_decl_body" src/wasm_backend.rs
grep -qF '        let lazy = |i: usize| self.demand.is_lazy_bind(&decl.name, arity, i);' "$f" || {
  echo "the lazy question in emit_decl_body moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let lazy = |i: usize| self.demand.is_lazy_bind(&decl.name, arity, i);$/        let lazy = |i: usize| false \&\& self.demand.is_lazy_bind(\&decl.name, arity, i);/' "$f"
grep -qF 'let lazy = |i: usize| false && self.demand' "$f"
