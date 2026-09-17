#!/bin/sh
# The compiler grows a shared object, which is the one thing a compiler change
# can do to move the half of startup compile_instructions.sh stopped counting.
#
# That gate reads the compiler's own work from `std::rt::lang_start::
# {{closure}}` down. Above it sit 465,122 instructions of loader and stack
# guard, and they move with where the linker happened to put things rather than
# with the front end — three values spanning 2,140 across four binaries whose
# sources differ only in a static nothing reads. Loading is the exception: one
# more shared object was measured at about 32,090, and dropping the frame drops
# that too.
#
# `-C prefer-dynamic` links Rust's standard library dynamically, so the binary
# grows a `libstd-<hash>.so` it did not have. It is a real dependency and a
# realistic one — it is how a distribution packages a Rust program — and no
# line of Rust changed to get it, which is the shape the instruction row could
# never have caught anyway.
#
# The repo grew its own .cargo/config.toml on 2026-09-17 -- it defines
# MI_NO_ALIGNED_HINT so the allocator stops picking random addresses -- and
# this mutation used to stop dead when it found one, because writing over a
# config it did not know about would have lost whatever was in it. So it
# appends instead, and refuses only if a `[build]` section is already there for
# it to collide with.
set -e
if [ -e .cargo/config ]; then
  echo "the repo grew a .cargo/config as well; this mutation needs rewriting" >&2
  exit 1
fi
if grep -q '^\[build\]' .cargo/config.toml 2>/dev/null; then
  echo "the cargo config already has a [build] section; this mutation would" >&2
  echo "have to merge with it and needs rewriting" >&2
  exit 1
fi
mkdir -p .cargo
cat >> .cargo/config.toml <<'EOF'

[build]
rustflags = ["-C", "prefer-dynamic"]
EOF
grep -q 'prefer-dynamic' .cargo/config.toml
