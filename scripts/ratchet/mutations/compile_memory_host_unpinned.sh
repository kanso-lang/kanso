#!/bin/sh
# What the front end holds is counted by its own allocator, so a good part of
# the figure is the standard library's growth schedules and it belongs to the
# rustc that built the binary — a container on 1.94.1 reads 864,274 where the
# runner on 1.98.0 reads 864,300. Moving the claim is the same as moving the
# toolchain: the gate has to refuse rather than measure and print a diff
# somebody will paste.
set -e
sed -i.bak 's/^# measured-on .*/# measured-on rustc=0.0.0/' \
  bench/compile_memory_golden.txt
rm -f bench/compile_memory_golden.txt.bak
grep -q '^# measured-on rustc=0.0.0' bench/compile_memory_golden.txt
