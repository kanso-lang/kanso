#!/bin/sh
# Rebuild the browser playground's interpreter. Prefers a rustup toolchain
# when the default cargo lacks the wasm target (the homebrew case); plain
# cargo works wherever rustup manages it (CI).
set -e
if cargo build --release --target wasm32-unknown-unknown 2>/dev/null; then
  :
else
  toolchain=$(ls -d "$HOME"/.rustup/toolchains/stable-* 2>/dev/null | head -1)
  RUSTC="$toolchain/bin/rustc" "$toolchain/bin/cargo" build --release --target wasm32-unknown-unknown
fi
cp target/wasm32-unknown-unknown/release/kanso.wasm docs/kanso.wasm
ls -la docs/kanso.wasm
