#!/bin/sh
# The .text rows belong to the toolchain that emitted them. Moving the claim is
# the same as moving the compiler: the gate has to refuse rather than measure
# and print a diff somebody will paste.
set -e
sed -i.bak 's/^# measured-on .*/# measured-on clang=3.4.2/' bench/text_golden.txt
rm -f bench/text_golden.txt.bak
grep -q '^# measured-on clang=3.4.2' bench/text_golden.txt
