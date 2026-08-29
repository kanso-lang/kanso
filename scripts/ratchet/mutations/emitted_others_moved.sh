#!/bin/sh
set -e
sed -i.bak 's/^scanbench defines=.*/scanbench defines=999999 calls=1 branches=1 lines=1/' bench/emitted_golden_others.txt
rm -f bench/emitted_golden_others.txt.bak
grep -q '^scanbench defines=999999' bench/emitted_golden_others.txt
