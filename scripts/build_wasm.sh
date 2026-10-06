#!/bin/sh
# Rebuild the browser playground's engine. Prefers a rustup toolchain
# when the default cargo lacks the wasm target (the homebrew case); plain
# cargo works wherever rustup manages it (CI).
#
# Only the binary is built: the library's cdylib shares its file name and
# would otherwise race it for target/.../kanso.wasm, and the binary is the
# one the page loads, since it carries the allocator's heap exports.
#
# Every visitor downloads this file, so its name section is stripped. That
# leaves the code byte for byte as it was and the browser rows unmoved. Fat
# LTO in one codegen unit would make it smaller still and is left off,
# because it makes the tab's compile dearer; the log entry "the engine
# drops its names" has the measurements.
set -e
CARGO_PROFILE_RELEASE_STRIP=true
export CARGO_PROFILE_RELEASE_STRIP
if cargo build --release --bin kanso --target wasm32-unknown-unknown 2>/dev/null; then
  :
else
  toolchain=$(ls -d "$HOME"/.rustup/toolchains/stable-* 2>/dev/null | head -1)
  RUSTC="$toolchain/bin/rustc" "$toolchain/bin/cargo" build --release --bin kanso --target wasm32-unknown-unknown
fi
cp target/wasm32-unknown-unknown/release/kanso.wasm docs/kanso.wasm
ls -la docs/kanso.wasm
