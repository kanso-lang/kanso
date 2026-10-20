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
#
# THE DIRECTORY LISTS ITS ENTRIES IN ONE ORDER. gold reads the directory in
# the order readdir returns it, and that order changes what the link costs: the
# same link of the same inputs read 22,443,938, 22,443,953 and 22,443,989 with
# these five entries listed three ways. ext4 lists a directory by a hash of
# each name seeded per filesystem, so every fresh runner could list the box its
# own way, and the dev row read 116,000,233 and 116,000,241 on commits that
# changed no code. tmpfs lists entries by when they were made, and the loop
# below makes them in a fixed order, so on Linux the entries live there and
# the box keeps its path to them.
if [ "$(uname -s)" = Linux ]; then
  if [ "$(stat -f -c %T /dev/shm 2>/dev/null)" != tmpfs ]; then
    echo "::error::/dev/shm is not a tmpfs here, so the link directory would list in the filesystem's own order" >&2
    exit 1
  fi
  rm -rf /dev/shm/kanso-codegen-link
  mkdir -p /dev/shm/kanso-codegen-link
  ln -s /dev/shm/kanso-codegen-link "$box/link"
else
  mkdir -p "$box/link"
fi
for lib in libm.so libgcc.a libgcc_s.so libgcc_s.so.1 libc.so; do
  found=$(env -i PATH=/usr/local/bin:/usr/bin:/bin clang -print-file-name="$lib")
  case "$found" in
    /*) ln -s "$found" "$box/link/$lib" ;;
    *) echo "::error::clang cannot find $lib, so the link directory is incomplete" >&2; exit 1 ;;
  esac
done
