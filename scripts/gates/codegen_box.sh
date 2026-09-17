#!/bin/sh
# Stage the compiler and the codegen corpus where the codegen rows are counted.
#
# THE PACKAGE SITS ONE LEVEL DOWN, and that is not tidiness. `kanso build X`
# names its output `X`, and it refuses outright when a directory of that name
# is beside it: "this build is named `codegen_corpus`, and a directory of that
# name is here". The refusal lands AFTER emit_ir has run, so a measurement
# taken that way looks plausible and counts a build that never wrote anything
# -- the first attempt at this row read two processes where a real build is
# five, with dev and release within 0.006% of each other. `bench/jsonbench`
# works for the same reason: the package is one level down from where the
# build runs.
set -e
box=/tmp/kanso-codegen
rm -rf "$box"
mkdir -p "$box/pkg"
cargo build --release
cp target/release/kanso "$box/kanso"
cp -r bench/codegen_corpus "$box/pkg/"
