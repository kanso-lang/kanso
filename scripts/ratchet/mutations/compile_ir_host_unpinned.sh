#!/bin/sh
# The instruction count belongs to the toolchain that built the binary and to
# the libc that serves its malloc and its memcpy. Moving the claim is the same
# as moving the host: the gate has to refuse rather than measure and print a
# diff somebody will paste.
set -e
sed -i.bak 's/^# measured-on .*/# measured-on glibc=0.0 rustc=0.0.0/' \
  bench/compile_instructions_golden.txt
rm -f bench/compile_instructions_golden.txt.bak
grep -q '^# measured-on glibc=0.0 rustc=0.0.0' bench/compile_instructions_golden.txt
