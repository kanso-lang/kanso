#!/bin/sh
# The empty list and map have inline copies in the release build's hot unit,
# and the LTO link inlines them into the decoder at every array and object it
# opens. This mutation puts noinline where their always_inline was, and the
# link leaves them as calls to the runtime's own copies. Same answers, and the
# work vein reads runbench heavier.
set -e
for k in list map; do
  t="__attribute__((always_inline)) KValue k_${k}_empty(void) {"
  [ "$(grep -cxF "$t" src/runtime/collections.c)" -eq 1 ] || {
    echo "the empty $k moved; this mutation needs rewriting" >&2
    exit 1
  }
  sed -i "s/^__attribute__((always_inline)) KValue k_${k}_empty(void) {\$/__attribute__((noinline)) KValue k_${k}_empty(void) {/" src/runtime/collections.c
  if grep -qxF "$t" src/runtime/collections.c; then exit 1; fi
  grep -qxF "__attribute__((noinline)) KValue k_${k}_empty(void) {" src/runtime/collections.c
done
