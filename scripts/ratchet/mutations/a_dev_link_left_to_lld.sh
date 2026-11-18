#!/bin/sh
# A dev build links with gold and without a build ID where gold links. This
# mutation hands every dev link back to what lld_args says, which on a Linux
# runner is lld with the driver's build ID, and the dev codegen row rises by
# the dynamic loader's work on libLLVM and by the SHA-1 of the output.
set -e
line='        return vec!["-fuse-ld=gold".to_string(), "-Wl,--build-id=none".to_string()];'
test "$(grep -cxF "$line" src/main/toolchain.rs)" = 1 || {
  echo "the dev link's choice of gold changed shape; rewrite this" >&2
  exit 1
}
awk -v line="$line" '$0 == line { print "        return lld_args();"; next } { print }' \
  src/main/toolchain.rs > src/main/toolchain.rs.new
mv src/main/toolchain.rs.new src/main/toolchain.rs
test "$(grep -cxF "$line" src/main/toolchain.rs)" = 0
