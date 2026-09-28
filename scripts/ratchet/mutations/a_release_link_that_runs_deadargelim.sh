#!/bin/sh
# Link a release build with LLVM's default pipeline, dead-argument elimination
# and all, as kanso did before 2026-09-28. The micro fixture
# a_subtype_beside_std_regexp_builds_in_release then fails to link, and
# micro_corpus_survives_a_release_build reports it.
set -e
grep -q '^        \.args(lto_pipeline_args())$' src/main.rs
sed -i.bak '/^        \.args(lto_pipeline_args())$/d' src/main.rs
rm -f src/main.rs.bak
! grep -q '^        \.args(lto_pipeline_args())$' src/main.rs
