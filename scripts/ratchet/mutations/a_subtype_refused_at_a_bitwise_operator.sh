#!/bin/sh
# A bitwise operator refuses a subtype's wrapper instead of reading its int.
#
# `&`, `|` and `^` reach the native runtime with a subtype operand still
# wrapped, where the bits builtins arrive already unwrapped. With the unwrap
# gone from k_bits_of, `id 1 & 3` for a `type id int` stops both native
# engines with "and takes whole numbers, got 1" while the interpreter answers
# 1, and the micro corpus reads the engines apart.
set -e
f=src/runtime.c
grep -q "static long long k_bits_of" src/runtime.c
line='    if (v.tag == K_SUB) v = k_sub_base(v);'
grep -A1 "static long long k_bits_of" "$f" | grep -qxF "$line" || {
  echo "the subtype unwrap in k_bits_of moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^static long long k_bits_of/{n;s/^    if (v.tag == K_SUB) v = k_sub_base(v);$/    \/* unwrap removed *\//}' "$f"
grep -A1 "static long long k_bits_of" "$f" | grep -qF '/* unwrap removed */'
