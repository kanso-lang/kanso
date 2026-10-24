#!/bin/sh
# The translator answers the leaves of a struct of two scalars, which is every
# %KValue, in place. This mutation lists them in a vector again, as every
# struct's were. Same leaves, and the browser compile row reads heavier.
set -e
t='            Ty::Struct(fs) if fs.len() == 2 && fs.iter().all(Ty::is_scalar) => {'
[ "$(grep -cxF "$t" src/ir_wasm.rs)" -eq 1 ] || {
  echo "the pair arm moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^            Ty::Struct(fs) if fs.len() == 2 && fs.iter().all(Ty::is_scalar) => {$/            Ty::Struct(fs) if fs.is_empty() \&\& fs.len() == 2 \&\& fs.iter().all(Ty::is_scalar) => {/' src/ir_wasm.rs
if grep -qxF "$t" src/ir_wasm.rs; then exit 1; fi
grep -qF 'Ty::Struct(fs) if fs.is_empty() && fs.len() == 2' src/ir_wasm.rs
