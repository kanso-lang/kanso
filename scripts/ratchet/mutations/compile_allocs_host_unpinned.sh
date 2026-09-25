#!/bin/sh
# The allocation vein belongs to the rustc that built the binary — a good part
# of the count is the standard library's. Moving the claim is the same as
# moving the toolchain: the gate has to refuse rather than measure and print a
# diff somebody will paste.
set -e
sed -i.bak 's/^# measured-on .*/# measured-on rustc=0.0.0/' \
  bench/compile_allocs_golden.txt
rm -f bench/compile_allocs_golden.txt.bak
grep -q '^# measured-on rustc=0.0.0' bench/compile_allocs_golden.txt
