#!/bin/sh
# Rebuild the browser playground's interpreter. The homebrew rust toolchain
# has no wasm target, so this pins the rustup one explicitly.
set -e
toolchain="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin"
RUSTC="$toolchain/bin/rustc" "$toolchain/bin/cargo" build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/kanso.wasm docs/kanso.wasm
ls -la docs/kanso.wasm
