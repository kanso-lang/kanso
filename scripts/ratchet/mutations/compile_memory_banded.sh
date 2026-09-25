#!/bin/sh
# The band was the bug's whole habitat: two per cent of 871,649 is 17,432 bytes
# of slack for a host divergence documented at tens of bytes, and main drifted
# 376 bytes greener inside it with CI green throughout. Clay ruled it away on
# 2026-08-24 — "that shouldn't be a tolerance. it should be a setting per
# platform." This puts a peak the compiler does not hold into the golden by the
# smallest amount the old band would have swallowed, so a gate that went back
# to reading a tolerance would pass it.
set -e
sed -i.bak 's/^compile_peak_bytes=.*/compile_peak_bytes=865300/' \
  bench/compile_memory_golden.txt
rm -f bench/compile_memory_golden.txt.bak
grep -q '^compile_peak_bytes=865300' bench/compile_memory_golden.txt
