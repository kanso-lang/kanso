#!/bin/sh
# The compiler's allocator is a performance claim like any other, and the
# claim it makes is large: glibc's malloc and free were 12.7% of
# `kanso check compile_corpus`, and putting them back costs about a tenth of
# all three compile rows. The counting wrapper above it is unchanged either
# way — the compiler's demand is the same and every counter the objective
# reads is byte-identical — so no gate but the instruction veins can see this
# at all. If the swap were ever undone by a merge or a revert, the counters
# would agree and the goldens would be the only thing left objecting.
set -e
sed -i.bak 's/^const UNDER: mimalloc::MiMalloc = mimalloc::MiMalloc;/const UNDER: std::alloc::System = std::alloc::System;/' \
  src/main.rs
rm -f src/main.rs.bak
grep -q '^const UNDER: std::alloc::System = std::alloc::System;$' src/main.rs
! grep -q 'const UNDER: mimalloc::MiMalloc' src/main.rs
