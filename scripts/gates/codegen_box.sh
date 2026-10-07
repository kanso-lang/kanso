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

# AND THE LINK SEARCHES ONE DIRECTORY, staged here, holding the five libraries
# the link resolves and nothing else. `ld.gold` reads each `-L` directory whole
# before it looks a library up, so the dev row counted how many files the
# runner image had installed in `/usr/lib/x86_64-linux-gnu`: one file added
# there moved it 3,559 instructions, and on CI it took two values on one
# binary, 130,465,044 and 130,465,609, that followed the image version and
# nothing in the tree. The gate hands this directory to kanso as
# KANSO_LINK_DIR, and the link job's `-L` list becomes this one entry.
#
# Each library is found the way the build's own clang finds it, under the
# environment the measurement runs in, and a name clang cannot resolve stops
# the staging: a link that finds nothing here fails, and a failed link is
# not a row.
mkdir -p "$box/link"
for lib in libm.so libgcc.a libgcc_s.so libgcc_s.so.1 libc.so; do
  found=$(env -i PATH=/usr/local/bin:/usr/bin:/bin clang -print-file-name="$lib")
  case "$found" in
    /*) ln -s "$found" "$box/link/$lib" ;;
    *) echo "::error::clang cannot find $lib, so the link directory is incomplete" >&2; exit 1 ;;
  esac
done
