#!/bin/sh
# A builder only one function holds grows by realloc, which extends or remaps
# the block and never holds the old buffer beside the new one. This mutation
# sends the grow back through malloc, copy and free. The bytes are the same;
# the witness is the fixture
# a_builder_that_outgrows_its_buffer_is_never_held_twice, whose
# held_peak_bytes counts both buffers again.
set -e
grep -qF '    if (mutate && !dies && k_bytes_malloced(a)) {' src/runtime.c || {
  echo "the in-place regrow moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    if (mutate \&\& !dies \&\& k_bytes_malloced(a)) {$|    if (0 \&\& mutate \&\& !dies \&\& k_bytes_malloced(a)) {|' src/runtime.c
[ "$(grep -cF '    if (0 && mutate && !dies && k_bytes_malloced(a)) {' src/runtime.c)" -eq 1 ]
