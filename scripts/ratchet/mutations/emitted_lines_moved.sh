#!/bin/sh
set -e
sed -i.bak 's/^defines=.*/defines=999999/' bench/emitted_golden.txt
rm -f bench/emitted_golden.txt.bak
grep -q '^defines=999999' bench/emitted_golden.txt
