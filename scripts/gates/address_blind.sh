#!/bin/sh
# Builds the address-blind memcmp every kanso-process instruction gate
# preloads, proves it is address-blind, and prints its path.
#
# The proof is the reason this is a script and not a flag: the same sixteen
# bytes are compared at page offset 64 and at 4080 under callgrind, counting
# only the frame that compares, and the two counts must agree. Under libc's
# memcmp they do not, which is what the gates are being protected from, and
# `KANSO_BLIND_SELFTEST_BARE=1` runs the probe without the preload so that the
# refusal can be watched.
set -e
here=$(dirname "$0")/address_blind
lib=/tmp/kanso-address-blind.so
probe=/tmp/kanso-address-blind-probe
cc -O2 -fno-builtin -fno-tree-loop-distribute-patterns -shared -fPIC -o "$lib" "$here/compare.c"
cc -O1 -o "$probe" "$here/probe.c"
count() {
  rm -f /tmp/cg.blind
  if [ -n "$KANSO_BLIND_SELFTEST_BARE" ]; then
    env -i valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.blind --toggle-collect=probe "$probe" "$1" >/dev/null 2>&1 || true
  else
    env -i LD_PRELOAD="$lib" valgrind --tool=callgrind --callgrind-out-file=/tmp/cg.blind --toggle-collect=probe "$probe" "$1" >/dev/null 2>&1 || true
  fi
  awk '/^summary:/ { print $2 }' /tmp/cg.blind
}
near=$(count 64)
edge=$(count 4080)
if [ -z "$near" ] || [ "$near" != "$edge" ]; then
  echo "::error::the preloaded memcmp is not address-blind: sixteen bytes cost $near at page offset 64 and $edge at 4080" >&2
  exit 1
fi
echo "$lib"
