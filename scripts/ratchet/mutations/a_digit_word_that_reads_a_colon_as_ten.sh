#!/bin/sh
# k_nondigits8 flags a byte that is not an ascii digit by adding 0x76 to its
# distance from '0': ten and above reach the top bit. One less and ':', the
# byte after '9', reads as a digit worth ten, so "1:" parses as twenty and a
# number with a colon in it is taken rather than refused.
set -e
t='    return (((t & 0x7F7F7F7F7F7F7F7FULL) + 0x7676767676767676ULL) | t) & 0x8080808080808080ULL;'
n=$(grep -cF "$t" src/runtime.c)
[ "$n" -eq 1 ] || { echo "the digit word's test changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/+ 0x7676767676767676ULL) | t)/+ 0x7575757575757575ULL) | t)/' src/runtime.c
grep -qF '+ 0x7575757575757575ULL) | t)' src/runtime.c
