#!/bin/sh
# Ask the token's word for bytes below 31 where the writer escapes those
# below 32.
#
# Byte 31 then reads as clean, and a token holding it is written raw.
set -e
sed -i.bak 's/((w - ones \* 32) \& ~w)/((w - ones * 31) \& ~w)/' src/runtime/text.c
rm -f src/runtime/text.c.bak
grep -q '((w - ones \* 31) & ~w)' src/runtime/text.c
