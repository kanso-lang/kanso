#!/bin/sh
set -e
sed -i.bak 's/^jsonbench text=.*/jsonbench text=999999/' bench/text_golden.txt
rm -f bench/text_golden.txt.bak
grep -q '^jsonbench text=999999' bench/text_golden.txt
