#!/bin/sh
# Build runtime.c for wasm32, as the module the playground links a compiled
# program against: `sh scripts/build_runtime_wasm.sh [out]`, out defaulting to
# docs/kanso-runtime.wasm.
#
# The playground compiles a program with the native emitter and lowers its
# module to wasm in the tab (src/ir_wasm.rs). That module is a side module: it
# imports this one's memory, table and stack pointer, and calls the same k_*
# functions a native binary calls. So this module exports everything, its
# table grows, and wasm/hooks.c gives it slots for the seven functions a
# native link would have resolved by name.
#
# What each flag is for:
# - experimental-mv when runtime.c is lowered to IR and not after. The emitted
#   module passes a KValue as two i64 words, which the multivalue ABI gives a
#   C struct too; the standard ABI stays on the object step, which is the ABI
#   wasi-libc's 128-bit helpers were built with.
# - wasm/include and wasm/shim.c answer the process and socket calls the
#   effect executor makes as failures, and supply three 128-bit helpers no
#   wasm32 compiler-rt on this toolchain has.
# - -mtail-call, because the emitted module's musttail calls stay tail calls.
# - An 8 MB stack placed first. wasm-ld's default is 64 KB growing down into
#   the data segment, and a twenty-step `.>` chain overran it.
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/docs/kanso-runtime.wasm}
sysroot=${WASI_SYSROOT:-/usr}
lib=${WASI_LIB:-/usr/lib/wasm32-wasi}
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
target="--target=wasm32-wasi --sysroot=$sysroot -O2 -mtail-call"
mv="-Xclang -target-abi -Xclang experimental-mv"
clang $target $mv -D_WASI_EMULATED_SIGNAL -w -S -emit-llvm -I "$root/wasm/include" \
  "$root/src/runtime.c" -o "$work/runtime.ll"
clang $target $mv -w -S -emit-llvm "$root/wasm/hooks.c" -o "$work/hooks.ll"
clang $target -w -c "$work/runtime.ll" -o "$work/runtime.o"
clang $target -w -c "$work/hooks.ll" -o "$work/hooks.o"
clang $target -w -c "$root/wasm/shim.c" -o "$work/shim.o"
wasm-ld "$lib/crt1-command.o" "$work/runtime.o" "$work/hooks.o" "$work/shim.o" \
  -L"$lib" -lc -lwasi-emulated-signal -lwasi-emulated-getpid \
  --export-all --export=__stack_pointer --export-table --growable-table --strip-all \
  --stack-first -z stack-size=8388608 -o "$out"
