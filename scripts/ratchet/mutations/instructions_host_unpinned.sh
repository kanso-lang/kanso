#!/bin/sh
# The rows of the instructions vein belong to the host that measured them.
# Moving the claim is the same as moving the box: the gate has to refuse
# rather than measure and print a diff somebody will paste.
set -e
sed -i.bak 's/^# measured-on .*/# measured-on glibc=2.28-0nowhere1/' \
  bench/instructions_golden.txt
rm -f bench/instructions_golden.txt.bak
grep -q '^# measured-on glibc=2.28-0nowhere1' bench/instructions_golden.txt
