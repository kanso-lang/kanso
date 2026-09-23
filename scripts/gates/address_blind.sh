#!/bin/sh
# Builds the address-blind string functions every kanso-process instruction
# gate preloads, proves them, and prints the library's path.
#
# Three proofs, in order, and the gate refuses on the first that fails:
#
#   - check.c runs memcmp, bcmp, memcpy and memmove against libc's own answers
#     on 80,000 random cases and on every length to 300 at every distance
#     from -140 to 140, overlapping moves in both directions included. A wrong
#     replacement would not show as a wrong count; it would corrupt the
#     compiler under measurement.
#   - probe.c compares the same sixteen bytes at page offset 64 and at 4080,
#     counting only the comparing frame. libc's avx2 memcmp takes a longer
#     branch near a page end.
#   - copy_probe.c moves the same 1,500 bytes to destinations 4096 and 5000
#     above the source. libc's memmove picks its path partly by that distance.
#
# `KANSO_BLIND_SELFTEST_BARE=1` runs the two probes without the preload, so
# that each refusal can be watched.
set -e
here=$(dirname "$0")/address_blind
lib=/tmp/kanso-address-blind.so
probe=/tmp/kanso-address-blind-probe
copy_probe=/tmp/kanso-address-blind-copy-probe
check=/tmp/kanso-address-blind-check
flags="-O2 -mavx2 -fno-builtin -fno-tree-loop-distribute-patterns"
cc $flags -shared -fPIC -o "$lib" "$here/compare.c" "$here/copy.c"
cc $flags -o "$check" "$here/check.c"
cc -O1 -o "$probe" "$here/probe.c"
cc -O1 -o "$copy_probe" "$here/copy_probe.c"
if ! "$check" >/dev/null; then
  echo "::error::the preloaded string functions disagree with libc; see the line above" >&2
  exit 1
fi
count() {
  rm -f /tmp/cg.blind
  if [ -n "$KANSO_BLIND_SELFTEST_BARE" ]; then
    env -i valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.blind --toggle-collect=probe "$1" "$2" >/dev/null 2>&1 || true
  else
    env -i LD_PRELOAD="$lib" valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.blind --toggle-collect=probe "$1" "$2" >/dev/null 2>&1 || true
  fi
  awk '/^summary:/ { print $2 }' /tmp/cg.blind
}
near=$(count "$probe" 64)
edge=$(count "$probe" 4080)
if [ -z "$near" ] || [ "$near" != "$edge" ]; then
  echo "::error::the preloaded memcmp is not address-blind: sixteen bytes cost $near at page offset 64 and $edge at 4080" >&2
  exit 1
fi
apart=$(count "$copy_probe" 4096)
further=$(count "$copy_probe" 5000)
if [ -z "$apart" ] || [ "$apart" != "$further" ]; then
  echo "::error::the preloaded memcpy is not address-blind: 1,500 bytes cost $apart at a distance of 4096 and $further at 5000" >&2
  exit 1
fi
echo "$lib"
