#!/bin/sh
# The library's encoder copies each string before it writes it: a slice of
# the whole string goes to `append_string` in place of the string itself.
#
# This is the kind of defect livebench was added for. bench/encodebench runs
# the same program four hundred times over and would not move by one
# instruction, because it vendors a snapshot of lib/json frozen at 20ab931d on
# purpose. bench/oneshot imports the real library and encodes exactly once, so
# it sees a sliver. The live vein sees all of it: one allocation per string
# written, 858,314 where it reads 8,314.
#
# The program still answers the same bytes, so the checksum gate stays green
# and the counters are what turn red. Until 2026-10-26 this repointed the
# escaper's clean-string arm, which the encoder stopped calling when strings
# went out through `append_string` in one call, and the row read nothing.
set -e
grep -q '^  builtin_append_string acc s$' lib/json/json.kso || {
  echo "lib/json's string writer changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's|^  builtin_append_string acc s$|  builtin_append_string acc (text/slice s 1 (length s))|' \
  lib/json/json.kso
grep -q '^  builtin_append_string acc (text/slice s 1 (length s))$' lib/json/json.kso
