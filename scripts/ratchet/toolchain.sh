#!/bin/sh
# What the ratchet needs on the box before it can read anybody's gate.
#
# The ratchet runs OTHER jobs' gates. Every other job in ci.yml installs what
# its own gates need and nothing else, which is right for that job and leaves
# the ratchet holding a base image. On 2026-09-03 the baseline pass added in
# kanso#1228 said so out loud on its first runtime-touching pull request:
#
#   ALREADY RED cost goldens (deterministic ratchet, no clocks)
#     gate: sh scripts/gates/instructions.sh
#     red before any mutation, so no row sharing it is proof
#
# scripts/gates/instructions.sh runs callgrind, and valgrind is installed at
# one place in ci.yml — inside the cost goldens job, four steps before that
# gate. Neither the nightly `prove` nor the per-pull-request `touched` step
# installed it, so both rows whose gate needs it were red before any mutation
# was applied, and the ratchet reads red as proof. They had proved nothing from
# the day they were written.
#
# So the ratchet's box carries everything any ci.yml job installs, gathered
# here rather than spread across two workflow files.
# tests/the_ratchet_carries_what_its_gates_need.rs is what keeps this a
# superset: a package added to any job in ci.yml and not added here turns the
# specs job red, because the row that needed it would otherwise go quietly
# blind exactly the way these two did.
set -e

# valgrind: scripts/gates/instructions.sh and scripts/gates/compile_instructions.sh.
# jq: kq's spec.sh compares byte-for-byte against `jq -S`, and without it that
# half of the comparison silently does not run.
sudo apt-get update -qq
sudo apt-get install -y -qq valgrind jq

# clang-19: the emitter probes for the preserve_none calling convention and
# writes the fallback when the probe fails, so a box with only clang 18 builds
# DIFFERENT binaries from the ones the cost goldens pin. The cost-goldens job
# installs this; the ratchet runs that job's gates, so it needs it too, or
# emitted_code and machine_code are red here before any mutation is applied --
# which is the false proof this whole file exists to prevent.
#
# --no-install-recommends is required: a plain install pulls llvm-19-dev, whose
# 32-bit dependencies (libc6-i386, libxml2-dev) 404 on this image.
sudo apt-get install -y -qq --no-install-recommends clang-19
sudo update-alternatives --install /usr/bin/clang clang /usr/bin/clang-19 100

# The browser rows' gates rebuild docs/kanso.wasm. ratchet.yml carried this
# line already and said why; the `touched` step in ci.yml never got it, so a
# branch touching src/wasm.rs would have failed the in_the_page row's gate on
# the build rather than on the defect — which is the same false proof by
# another route.
rustup target add wasm32-unknown-unknown
